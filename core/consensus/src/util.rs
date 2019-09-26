use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use creep::Context;
use overlord::{types::AggregatedSignature, Crypto};

use protocol::traits::{MessageHandler, Priority, Rpc, Storage};
use protocol::types::{Hash, UserAddress};
use protocol::{ProtocolError, ProtocolResult};

use common_crypto::{
    Crypto as Secp256k1Crypto, PrivateKey, PublicKey, Secp256k1, Secp256k1PrivateKey,
    Secp256k1PublicKey, Signature,
};

use crate::fixed_types::{ConsensusRpcRequest, FixedEpochs, FixedSignedTxs};
use crate::message::RPC_SYNC_PULL;
use crate::ConsensusError;

#[derive(Clone, Debug)]
pub struct OverlordCrypto {
    public_key:  Secp256k1PublicKey,
    private_key: Secp256k1PrivateKey,
}

impl Crypto for OverlordCrypto {
    fn hash(&self, msg: Bytes) -> Bytes {
        Hash::digest(msg).as_bytes()
    }

    fn sign(&self, hash: Bytes) -> Result<Bytes, Box<dyn Error + Send>> {
        let signature = Secp256k1::sign_message(&hash, &self.private_key.to_bytes())
            .map_err(|e| ProtocolError::from(ConsensusError::CryptoErr(Box::new(e))))?
            .to_bytes();

        let mut res = self.public_key.to_bytes();
        res.extend_from_slice(&signature);
        Ok(res)
    }

    fn verify_signature(
        &self,
        mut signature: Bytes,
        hash: Bytes,
    ) -> Result<Bytes, Box<dyn Error + Send>> {
        let tmp = signature.split_off(33);
        let pub_key = signature;
        let signature = tmp;

        Secp256k1::verify_signature(&hash, &signature, &pub_key)
            .map_err(|e| ProtocolError::from(ConsensusError::CryptoErr(Box::new(e))))?;
        let address = UserAddress::from_pubkey_bytes(pub_key)?;
        Ok(address.as_bytes())
    }

    fn aggregate_signatures(
        &self,
        _signatures: Vec<Bytes>,
        _voters: Vec<Bytes>,
    ) -> Result<Bytes, Box<dyn Error + Send>> {
        Ok(Bytes::new())
    }

    fn verify_aggregated_signature(
        &self,
        _aggregated_signature: AggregatedSignature,
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

impl OverlordCrypto {
    pub fn new(public_key: Secp256k1PublicKey, private_key: Secp256k1PrivateKey) -> Self {
        OverlordCrypto {
            public_key,
            private_key,
        }
    }
}

#[derive(Debug)]
pub struct RpcHandler<R, S> {
    rpc:     Arc<R>,
    storage: Arc<S>,
}

#[async_trait]
impl<R: Rpc + 'static, S: Storage + 'static> MessageHandler for RpcHandler<R, S> {
    type Message = ConsensusRpcRequest;

    async fn process(&self, ctx: Context, msg: ConsensusRpcRequest) -> ProtocolResult<()> {
        match msg {
            ConsensusRpcRequest::PullEpochs(ep) => {
                let res = self.storage.get_epoch_by_epoch_id(ep.inner).await?;

                self.rpc
                    .response(ctx, RPC_SYNC_PULL, FixedEpochs::new(res), Priority::High)
                    .await
            }

            ConsensusRpcRequest::PullTxs(txs) => {
                let mut res = Vec::new();
                for tx in txs.inner.into_iter() {
                    res.push(self.storage.get_transaction_by_hash(tx).await?);
                }
                self.rpc
                    .response(ctx, RPC_SYNC_PULL, FixedSignedTxs::new(res), Priority::High)
                    .await
            }
        }
    }
}

impl<R, S> RpcHandler<R, S>
where
    R: Rpc + 'static,
    S: Storage + 'static,
{
    #![allow(dead_code)]
    pub fn new(rpc: Arc<R>, storage: Arc<S>) -> Self {
        RpcHandler { rpc, storage }
    }
}
