use std::sync::Arc;

use super::test::Bencher;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use cita_trie::MemoryDB;

use asset::AssetService;
use protocol::traits::{Executor, ExecutorParams, Service, ServiceMapping, ServiceSDK, Storage};
use protocol::types::{
    Block, Genesis, Hash, Proof, RawTransaction, Receipt, SignedTransaction, TransactionRequest,
};
use protocol::ProtocolResult;

use super::test_service::TestService;
use crate::executor::ServiceExecutor;

#[bench]
fn bench_with_tx_hook(b: &mut Bencher) {
    let toml_str = include_str!("./genesis_services.toml");
    let genesis: Genesis = toml::from_str(toml_str).unwrap();

    let db = Arc::new(MemoryDB::new(false));

    let root = ServiceExecutor::create_genesis(
        genesis.services,
        Arc::clone(&db),
        Arc::new(MockStorage {}),
        Arc::new(MockServiceMapping {}),
    )
    .unwrap();

    let mut executor = ServiceExecutor::with_root(
        root.clone(),
        Arc::clone(&db),
        Arc::new(MockStorage {}),
        Arc::new(MockServiceMapping {}),
    )
    .unwrap();

    let txs: Vec<SignedTransaction> = (0..10000).map(|_| mock_signed_tx()).collect();

    b.iter(|| {
        let params = ExecutorParams {
            state_root:   root.clone(),
            height:       1,
            timestamp:    0,
            cycles_limit: std::u64::MAX,
        };
        let txs = txs.clone();
        executor.exec(&params, &txs).unwrap();
    });
}

struct MockServiceMapping;

impl ServiceMapping for MockServiceMapping {
    fn get_service<SDK: 'static + ServiceSDK>(
        &self,
        name: &str,
        sdk: SDK,
    ) -> ProtocolResult<Box<dyn Service>> {
        let service = match name {
            "asset" => Box::new(AssetService::new(sdk)) as Box<dyn Service>,
            "test11" => Box::new(TestService::new(sdk)) as Box<dyn Service>,
            "test12" => Box::new(TestService::new(sdk)) as Box<dyn Service>,
            "test13" => Box::new(TestService::new(sdk)) as Box<dyn Service>,
            "test14" => Box::new(TestService::new(sdk)) as Box<dyn Service>,
            "test15" => Box::new(TestService::new(sdk)) as Box<dyn Service>,
            "test1" => Box::new(TestService::new(sdk)) as Box<dyn Service>,
            _ => panic!("not found service"),
        };

        Ok(service)
    }

    fn list_service_name(&self) -> Vec<String> {
        vec![
            "asset".to_owned(),
            "test11".to_owned(),
            "test12".to_owned(),
            "test13".to_owned(),
            "test14".to_owned(),
            "test15".to_owned(),
            "test1".to_owned(),
        ]
    }
}

struct MockStorage;

#[async_trait]
impl Storage for MockStorage {
    async fn insert_transactions(&self, _: Vec<SignedTransaction>) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn insert_block(&self, _: Block) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn insert_receipts(&self, _: Vec<Receipt>) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn update_latest_proof(&self, _: Proof) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn get_transaction_by_hash(&self, _: Hash) -> ProtocolResult<SignedTransaction> {
        unimplemented!()
    }

    async fn get_transactions(&self, _: Vec<Hash>) -> ProtocolResult<Vec<SignedTransaction>> {
        unimplemented!()
    }

    async fn get_latest_block(&self) -> ProtocolResult<Block> {
        unimplemented!()
    }

    async fn get_block_by_height(&self, _: u64) -> ProtocolResult<Block> {
        unimplemented!()
    }

    async fn get_block_by_hash(&self, _: Hash) -> ProtocolResult<Block> {
        unimplemented!()
    }

    async fn get_receipt(&self, _: Hash) -> ProtocolResult<Receipt> {
        unimplemented!()
    }

    async fn get_receipts(&self, _: Vec<Hash>) -> ProtocolResult<Vec<Receipt>> {
        unimplemented!()
    }

    async fn get_latest_proof(&self) -> ProtocolResult<Proof> {
        unimplemented!()
    }

    async fn update_overlord_wal(&self, _info: Bytes) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn load_overlord_wal(&self) -> ProtocolResult<Bytes> {
        unimplemented!()
    }
}

fn mock_signed_tx() -> SignedTransaction {
    let raw = RawTransaction {
        chain_id:     Hash::from_empty(),
        nonce:        Hash::from_empty(),
        timeout:      0,
        cycles_price: 1,
        cycles_limit: std::u64::MAX,
        request:      TransactionRequest {
            service_name: "asset".to_owned(),
            method:       "create_asset".to_owned(),
            payload:      r#"{ "name": "MutaToken2", "symbol": "MT2", "supply": 320000011 }"#
                .to_owned(),
        },
    };

    SignedTransaction {
        raw,
        tx_hash: Hash::from_empty(),
        pubkey: Bytes::from(
            hex::decode("031288a6788678c25952eba8693b2f278f66e2187004b64ac09416d07f83f96d5b")
                .unwrap(),
        ),
        signature: BytesMut::from("").freeze(),
    }
}
