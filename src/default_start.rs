use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;

use bytes::Bytes;
use parking_lot::RwLock;

use common_crypto::{
    BlsCommonReference, BlsPrivateKey, BlsPublicKey, PublicKey, Secp256k1, Secp256k1PrivateKey,
    ToPublicKey,
};
use core_api::adapter::DefaultAPIAdapter;
use core_api::config::GraphQLConfig;
use core_consensus::fixed_types::{FixedEpoch, FixedSignedTxs};
use core_consensus::message::{
    ProposalMessageHandler, PullEpochRpcHandler, PullTxsRpcHandler, QCMessageHandler,
    RichEpochIDMessageHandler, VoteMessageHandler, END_GOSSIP_AGGREGATED_VOTE,
    END_GOSSIP_RICH_EPOCH_ID, END_GOSSIP_SIGNED_PROPOSAL, END_GOSSIP_SIGNED_VOTE,
    RPC_RESP_SYNC_PULL_EPOCH, RPC_RESP_SYNC_PULL_TXS, RPC_SYNC_PULL_EPOCH, RPC_SYNC_PULL_TXS,
};
use core_consensus::status::{CurrentConsensusStatus, StatusPivot};
use core_consensus::{OverlordConsensus, OverlordConsensusAdapter};
use core_mempool::{
    DefaultMemPoolAdapter, HashMemPool, MsgPushTxs, NewTxsHandler, PullTxsHandler,
    END_GOSSIP_NEW_TXS, RPC_PULL_TXS, RPC_RESP_PULL_TXS,
};
use core_network::{NetworkConfig, NetworkService};
use core_storage::{adapter::rocks::RocksAdapter, ImplStorage};
use framework::binding::state::RocksTrieDB;
use framework::executor::{ServiceExecutor, ServiceExecutorFactory};
use protocol::traits::{NodeInfo, ServiceMapping, Storage};
use protocol::types::{Address, Bloom, Epoch, EpochHeader, Genesis, Hash, Proof, Validator};
use protocol::{fixed_codec::FixedCodec, ProtocolError, ProtocolResult};

use crate::config::Config;
use crate::MainError;

pub async fn create_genesis<Mapping: 'static + ServiceMapping>(
    config: &Config,
    genesis: &Genesis,
    servive_mapping: Arc<Mapping>,
) -> ProtocolResult<Epoch> {
    let chain_id = Hash::from_hex(&config.chain_id)?;

    // Read genesis.
    log::info!("Genesis data: {:?}", genesis);

    // Init Block db
    let path_block = config.data_path_for_block();
    let rocks_adapter = Arc::new(RocksAdapter::new(path_block)?);
    let storage = Arc::new(ImplStorage::new(Arc::clone(&rocks_adapter)));

    match storage.get_latest_epoch().await {
        Ok(genesis_epoch) => {
            log::info!("The Genesis block has been initialized.");
            return Ok(genesis_epoch);
        }
        Err(e) => {
            if !e.to_string().contains("GetNone") {
                return Err(e);
            }
        }
    };

    // Init trie db
    let path_state = config.data_path_for_state();
    let trie_db = Arc::new(RocksTrieDB::new(path_state, config.executor.light)?);

    // Init genesis
    let genesis_state_root = ServiceExecutor::create_genesis(
        genesis.services.clone(),
        Arc::clone(&trie_db),
        Arc::clone(&storage),
        servive_mapping,
    )?;

    // Build genesis block.
    let genesis_epoch_header = EpochHeader {
        chain_id:          chain_id.clone(),
        epoch_id:          0,
        pre_hash:          Hash::from_empty(),
        timestamp:         genesis.timestamp,
        logs_bloom:        vec![Bloom::default()],
        order_root:        Hash::from_empty(),
        confirm_root:      vec![],
        state_root:        genesis_state_root,
        receipt_root:      vec![Hash::from_empty()],
        cycles_used:       vec![0],
        proposer:          Address::from_hex("0000000000000000000000000000000000000000")?,
        proof:             Proof {
            epoch_id:   0,
            round:      0,
            epoch_hash: Hash::from_empty(),
            signature:  Bytes::new(),
            bitmap:     Bytes::new(),
        },
        validator_version: 0,
        validators:        vec![],
    };
    let latest_proof = genesis_epoch_header.proof.clone();
    let genesis_epoch = Epoch {
        header:            genesis_epoch_header,
        ordered_tx_hashes: vec![],
    };
    storage.insert_epoch(genesis_epoch.clone()).await?;
    storage.update_latest_proof(latest_proof).await?;

    log::info!("The genesis block is created {:?}", genesis_epoch);
    Ok(genesis_epoch)
}

pub async fn start<Mapping: 'static + ServiceMapping>(
    config: Config,
    service_mapping: Arc<Mapping>,
) -> ProtocolResult<()> {
    let chain_id = Hash::from_hex(&config.chain_id)?;

    // self private key
    let hex_privkey = hex::decode(config.privkey.clone()).map_err(MainError::FromHex)?;
    let my_privkey =
        Secp256k1PrivateKey::try_from(hex_privkey.as_ref()).map_err(MainError::Crypto)?;
    let my_pubkey = my_privkey.pub_key();
    let my_address = Address::from_pubkey_bytes(my_pubkey.to_bytes())?;

    // Init Block db
    let path_block = config.data_path_for_block();
    log::info!("Data path for block: {:?}", path_block);
    let rocks_adapter = Arc::new(RocksAdapter::new(path_block)?);
    let storage = Arc::new(ImplStorage::new(Arc::clone(&rocks_adapter)));

    // Init network
    let network_config = NetworkConfig::new();
    let network_privkey = config.privkey.clone();

    let mut bootstrap_pairs = vec![];
    if let Some(bootstrap) = &config.network.bootstraps {
        for bootstrap in bootstrap.iter() {
            bootstrap_pairs.push((bootstrap.pubkey.to_owned(), bootstrap.address));
        }
    }

    let network_config = network_config
        .bootstraps(bootstrap_pairs)?
        .secio_keypair(network_privkey)?;
    let mut network_service = NetworkService::new(network_config);
    network_service.listen(config.network.listening_address)?;

    // Init mempool
    let current_epoch = storage.get_latest_epoch().await?;
    let mempool_adapter = DefaultMemPoolAdapter::<Secp256k1, _, _>::new(
        network_service.handle(),
        Arc::clone(&storage),
        config.mempool.timeout_gap,
        config.mempool.broadcast_txs_size,
        config.mempool.broadcast_txs_interval,
    );
    let mempool = Arc::new(HashMemPool::new(
        config.mempool.pool_size as usize,
        config.mempool.timeout_gap,
        mempool_adapter,
    ));

    // register broadcast new transaction
    network_service.register_endpoint_handler(
        END_GOSSIP_NEW_TXS,
        Box::new(NewTxsHandler::new(Arc::clone(&mempool))),
    )?;

    // register pull txs from other node
    network_service.register_endpoint_handler(
        RPC_PULL_TXS,
        Box::new(PullTxsHandler::new(
            Arc::new(network_service.handle()),
            Arc::clone(&mempool),
        )),
    )?;
    network_service.register_rpc_response::<MsgPushTxs>(RPC_RESP_PULL_TXS)?;

    // Init trie db
    let path_state = config.data_path_for_state();
    let trie_db = Arc::new(RocksTrieDB::new(path_state, config.executor.light)?);

    // Init Consensus
    let node_info = NodeInfo {
        chain_id:     chain_id.clone(),
        self_address: my_address.clone(),
    };
    let current_header = &current_epoch.header;
    let prevhash = Hash::digest(current_epoch.encode_fixed()?);

    let current_consensus_status = Arc::new(RwLock::new(CurrentConsensusStatus {
        cycles_price:       config.consensus.cycles_price,
        cycles_limit:       config.consensus.cycles_limit,
        epoch_id:           current_epoch.header.epoch_id + 1,
        exec_epoch_id:      current_epoch.header.epoch_id,
        prev_hash:          prevhash,
        logs_bloom:         current_header.logs_bloom.clone(),
        confirm_root:       vec![],
        state_root:         vec![current_header.state_root.clone()],
        receipt_root:       vec![],
        cycles_used:        current_header.cycles_used.clone(),
        proof:              current_header.proof.clone(),
        validators:         config
            .consensus
            .verifier_list
            .iter()
            .map(|v| {
                Ok(Validator {
                    address:        Address::from_hex(v)?,
                    propose_weight: 1,
                    vote_weight:    1,
                })
            })
            .collect::<Result<Vec<Validator>, ProtocolError>>()?,
        consensus_interval: config.consensus.interval,
    }));

    assert!(config.consensus.verifier_list.len() == config.consensus.public_keys.len());
    let mut bls_pub_keys = HashMap::new();
    for (addr, bls_pub_key) in config
        .consensus
        .verifier_list
        .iter()
        .zip(config.consensus.public_keys.iter())
    {
        let address = Address::from_hex(addr)?.as_bytes();
        let hex_pubkey = hex::decode(bls_pub_key).map_err(MainError::FromHex)?;
        let pub_key = BlsPublicKey::try_from(hex_pubkey.as_ref()).map_err(MainError::Crypto)?;
        bls_pub_keys.insert(address, pub_key);
    }

    let hex_privkey =
        hex::decode(config.consensus.private_key.clone()).map_err(MainError::FromHex)?;
    let bls_priv_key = BlsPrivateKey::try_from(hex_privkey.as_ref()).map_err(MainError::Crypto)?;

    let hex_common_ref =
        hex::decode(config.consensus.common_ref.as_str()).map_err(MainError::FromHex)?;
    let common_ref: BlsCommonReference = std::str::from_utf8(hex_common_ref.as_ref())
        .map_err(MainError::Utf8)?
        .into();

    core_consensus::trace::init_tracer(my_address.as_hex())?;

    let (status_pivot, agent) = StatusPivot::new(Arc::clone(&current_consensus_status));

    let mut consensus_adapter =
        OverlordConsensusAdapter::<ServiceExecutorFactory, _, _, _, _, _, _>::new(
            Arc::new(network_service.handle()),
            Arc::new(network_service.handle()),
            Arc::clone(&mempool),
            Arc::clone(&storage),
            Arc::clone(&trie_db),
            Arc::clone(&service_mapping),
            agent,
            current_header.state_root.clone(),
        );

    let exec_demon = consensus_adapter.take_exec_demon();
    let consensus_adapter = Arc::new(consensus_adapter);

    let (tmp, synchronization) = OverlordConsensus::new(
        current_consensus_status,
        node_info,
        bls_pub_keys,
        bls_priv_key,
        common_ref,
        consensus_adapter,
    );

    let overlord_consensus = Arc::new(tmp);

    // register consensus
    network_service.register_endpoint_handler(
        END_GOSSIP_SIGNED_PROPOSAL,
        Box::new(ProposalMessageHandler::new(Arc::clone(&overlord_consensus))),
    )?;
    network_service.register_endpoint_handler(
        END_GOSSIP_AGGREGATED_VOTE,
        Box::new(QCMessageHandler::new(Arc::clone(&overlord_consensus))),
    )?;
    network_service.register_endpoint_handler(
        END_GOSSIP_SIGNED_VOTE,
        Box::new(VoteMessageHandler::new(Arc::clone(&overlord_consensus))),
    )?;
    network_service.register_endpoint_handler(
        END_GOSSIP_RICH_EPOCH_ID,
        Box::new(RichEpochIDMessageHandler::new(Arc::clone(
            &overlord_consensus,
        ))),
    )?;
    network_service.register_endpoint_handler(
        RPC_SYNC_PULL_EPOCH,
        Box::new(PullEpochRpcHandler::new(
            Arc::new(network_service.handle()),
            Arc::clone(&storage),
        )),
    )?;
    network_service.register_endpoint_handler(
        RPC_SYNC_PULL_TXS,
        Box::new(PullTxsRpcHandler::new(
            Arc::new(network_service.handle()),
            Arc::clone(&storage),
        )),
    )?;
    network_service.register_rpc_response::<FixedEpoch>(RPC_RESP_SYNC_PULL_EPOCH)?;
    network_service.register_rpc_response::<FixedSignedTxs>(RPC_RESP_SYNC_PULL_TXS)?;

    // Run network
    runtime::spawn(network_service);

    // Init graphql
    let api_adapter = DefaultAPIAdapter::<ServiceExecutorFactory, _, _, _, _>::new(
        Arc::clone(&mempool),
        Arc::clone(&storage),
        Arc::clone(&trie_db),
        Arc::clone(&service_mapping),
    );
    let mut graphql_config = GraphQLConfig::default();
    graphql_config.listening_address = config.graphql.listening_address;
    graphql_config.graphql_uri = config.graphql.graphql_uri.clone();
    graphql_config.graphiql_uri = config.graphql.graphiql_uri.clone();

    // Run GraphQL server
    runtime::spawn(core_api::start_graphql(graphql_config, api_adapter));

    // Run sychronization process
    runtime::spawn(synchronization.run());

    // Run status cache pivot
    runtime::spawn(status_pivot.run());

    // Run consensus
    runtime::spawn(async move {
        if let Err(e) = overlord_consensus
            .run(
                config.consensus.interval,
                Some(config.consensus.duration.clone()),
            )
            .await
        {
            log::error!("muta-consensus: {:?} error", e);
        }
    });

    // Run execute demon
    futures::executor::block_on(exec_demon.run());
    Ok(())
}
