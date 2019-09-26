use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bincode::deserialize;
use creep::Context;
use futures_timer::Delay;
use overlord::types::{AggregatedVote, Node, OverlordMsg, SignedProposal, SignedVote, Status};
use overlord::{DurationConfig, Overlord, OverlordHandler};
use parking_lot::RwLock;

use common_crypto::{PrivateKey, Secp256k1PrivateKey};

use protocol::traits::{Consensus, ConsensusAdapter, CurrentConsensusStatus, NodeInfo};
use protocol::types::{Proof, Validator};
use protocol::ProtocolResult;

use crate::engine::ConsensusEngine;
use crate::fixed_types::{FixedEpochID, FixedPill, FixedSignedTxs};
use crate::util::OverlordCrypto;
use crate::{ConsensusError, MsgType};

const BROADCAST_STATUS_INTERVAL: u64 = 3000;

/// Provide consensus
#[allow(dead_code)]
pub struct OverlordConsensus<Adapter: ConsensusAdapter + 'static> {
    /// Overlord consensus protocol instance.
    inner: Arc<Overlord<FixedPill, FixedSignedTxs, ConsensusEngine<Adapter>, OverlordCrypto>>,
    /// An overlord consensus protocol handler.
    handler: OverlordHandler<FixedPill>,
    /// A consensus engine for synchronous.
    engine: Arc<ConsensusEngine<Adapter>>,
}

#[async_trait]
impl<Adapter: ConsensusAdapter + 'static> Consensus for OverlordConsensus<Adapter> {
    async fn set_proposal(&self, ctx: Context, proposal: Vec<u8>) -> ProtocolResult<()> {
        let signed_proposal: SignedProposal<FixedPill> = rlp::decode(&proposal)
            .map_err(|_| ConsensusError::DecodeErr(MsgType::SignedProposal))?;
        self.handler
            .send_msg(ctx, OverlordMsg::SignedProposal(signed_proposal))
            .map_err(|e| ConsensusError::OverlordErr(Box::new(e)))?;
        Ok(())
    }

    async fn set_vote(&self, ctx: Context, vote: Vec<u8>) -> ProtocolResult<()> {
        let signed_vote: SignedVote =
            rlp::decode(&vote).map_err(|_| ConsensusError::DecodeErr(MsgType::SignedVote))?;
        self.handler
            .send_msg(ctx, OverlordMsg::SignedVote(signed_vote))
            .map_err(|e| ConsensusError::OverlordErr(Box::new(e)))?;
        Ok(())
    }

    async fn set_qc(&self, ctx: Context, qc: Vec<u8>) -> ProtocolResult<()> {
        let aggregated_vote: AggregatedVote =
            rlp::decode(&qc).map_err(|_| ConsensusError::DecodeErr(MsgType::AggregateVote))?;
        self.handler
            .send_msg(ctx, OverlordMsg::AggregatedVote(aggregated_vote))
            .map_err(|e| ConsensusError::OverlordErr(Box::new(e)))?;
        Ok(())
    }

    async fn update_epoch(&self, ctx: Context, msg: Vec<u8>) -> ProtocolResult<()> {
        // Reveive the rich epoch ID.
        let epoch_id: FixedEpochID =
            deserialize(&msg).map_err(|_| ConsensusError::DecodeErr(MsgType::RichEpochID))?;
        let mut current_epoch_id = self.engine.get_current_epoch_id(ctx.clone()).await?;
        let rich_epoch_id = epoch_id.inner;

        if current_epoch_id >= rich_epoch_id {
            return Ok(());
        } else if current_epoch_id == rich_epoch_id - 1 {
            // Wait for an interval and re-check if needs to synchronization.
            Delay::new(Duration::from_millis(BROADCAST_STATUS_INTERVAL))
                .await
                .map_err(|err| ConsensusError::Other(format!("{:?}", err)))?;

            current_epoch_id = self.engine.get_current_epoch_id(ctx.clone()).await?;
            if rich_epoch_id <= current_epoch_id {
                return Ok(());
            }
        }

        // Need to synchronization.
        self.engine.lock.lock().await;
        let mut prev_hash = self
            .engine
            .get_epoch_by_id(ctx.clone(), current_epoch_id + 1)
            .await?
            .header
            .pre_hash;

        for id in (current_epoch_id + 1)..(rich_epoch_id + 1) {
            // First pull a new block.
            let epoch = self.engine.pull_epoch(ctx.clone(), id).await?;
            let tmp_prev_hash = epoch.header.pre_hash.clone();

            // Check proof and previous hash.
            self.check_proof(id, epoch.header.proof.clone())?;
            if prev_hash != epoch.header.pre_hash {
                return Err(ConsensusError::SyncEpochHashErr(id).into());
            }

            // Then pull signed transactions.
            let txs = self
                .engine
                .pull_txs(ctx.clone(), epoch.ordered_tx_hashes.clone())
                .await?;

            // After get the signed transactions:
            // 1. Execute the signed transactions.
            // 2. Save the signed transactions.
            // 3. Save the latest proof.
            // 4. Save the new epoch.
            let _ = self.engine.exec(txs.clone()).await?;
            self.engine.save_signed_txs(txs).await?;
            self.engine.save_proof(epoch.header.proof.clone()).await?;
            self.engine.save_epoch(epoch).await?;

            // Update the previous hash for next check.
            prev_hash = tmp_prev_hash;
        }

        Ok(())
    }
}

impl<Adapter: ConsensusAdapter + 'static> OverlordConsensus<Adapter> {
    pub fn new(
        current_consensus_status: CurrentConsensusStatus,
        node_info: NodeInfo,
        priv_key: Secp256k1PrivateKey,
        adapter: Arc<Adapter>,
    ) -> Self {
        let current_consensus_status = Arc::new(RwLock::new(current_consensus_status));

        let engine = Arc::new(ConsensusEngine::new(
            Arc::clone(&current_consensus_status),
            node_info.clone(),
            Arc::clone(&adapter),
        ));

        let crypto = OverlordCrypto::new(priv_key.pub_key(), priv_key);
        let overlord = Overlord::new(
            node_info.self_address.as_bytes(),
            Arc::clone(&engine),
            crypto,
        );
        let overlord_handler = overlord.get_handler();

        overlord_handler
            .send_msg(
                Context::new(),
                OverlordMsg::RichStatus(gen_overlord_status(
                    current_consensus_status.read().epoch_id,
                    current_consensus_status.read().consensus_interval,
                    current_consensus_status.read().validators.clone(),
                )),
            )
            .unwrap();

        Self {
            inner: Arc::new(overlord),
            handler: overlord_handler,
            engine,
        }
    }

    pub async fn run(
        &self,
        interval: u64,
        timer_config: Option<DurationConfig>,
    ) -> ProtocolResult<()> {
        self.inner
            .run(interval, timer_config)
            .await
            .map_err(|e| ConsensusError::OverlordErr(Box::new(e)))?;

        Ok(())
    }

    fn check_proof(&self, _epoch_id: u64, _proof: Proof) -> ProtocolResult<()> {
        Ok(())
    }
}

fn gen_overlord_status(epoch_id: u64, interval: u64, validators: Vec<Validator>) -> Status {
    let mut authority_list = validators
        .into_iter()
        .map(|v| Node {
            address:        v.address.as_bytes(),
            propose_weight: v.propose_weight,
            vote_weight:    v.vote_weight,
        })
        .collect::<Vec<_>>();

    authority_list.sort();
    Status {
        epoch_id,
        interval: Some(interval),
        authority_list,
    }
}
