use std::convert::TryFrom;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use cita_trie::MemoryDB;

use common_crypto::{HashValue, PrivateKey, Secp256k1PrivateKey, Signature};
use framework::executor::ServiceExecutor;
use protocol::traits::{Executor, ExecutorParams, Service, ServiceMapping, ServiceSDK, Storage};
use protocol::types::{
    Address, Block, BlockHookReceipt, Genesis, Hash, Hex, Proof, RawTransaction, Receipt,
    SignedTransaction, TransactionRequest,
};
use protocol::ProtocolResult;

use crate::types::{BatchMintSudt, CKBMessage, MintSudt};
use crate::CKBHandler;
use ckb_sudt::CKBSudt;

#[test]
fn test_submit_message() {
    let (mut executor, params) = mock_executor_and_params();
    let raw_tx = RawTransaction {
        chain_id:     Hash::from_empty(),
        nonce:        Hash::from_empty(),
        timeout:      0,
        cycles_price: 1,
        cycles_limit: 60_000,
        request:      TransactionRequest {
            service_name: "ckb_handler".to_owned(),
            method:       "submit_message".to_owned(),
            payload:      mock_ckb_message(),
        },
    };
    let signed_tx = SignedTransaction {
        raw:       raw_tx,
        tx_hash:   Hash::from_empty(),
        pubkey:    Bytes::from(
            hex::decode("031288a6788678c25952eba8693b2f278f66e2187004b64ac09416d07f83f96d5b")
                .unwrap(),
        ),
        signature: BytesMut::from("").freeze(),
    };
    let txs = vec![signed_tx];
    let executor_resp = executor.exec(&params, &txs).unwrap();
    let receipt = &executor_resp.receipts[0];
    let response = &receipt.response.response;
    let events = &receipt.events;
    assert_eq!(response.is_error(), false);
    assert_eq!(3, events.len());
    assert_eq!("MintSudt", events[0].topic);
    assert_eq!("MessageSubmittedEvent", events[2].topic);
}

#[test]
fn test_set_relayer() {
    let (mut executor, params) = mock_executor_and_params();
    let raw_tx = RawTransaction {
        chain_id:     Hash::from_empty(),
        nonce:        Hash::from_empty(),
        timeout:      0,
        cycles_price: 1,
        cycles_limit: 60_000,
        request:      TransactionRequest {
            service_name: "ckb_handler".to_owned(),
            method:       "set_relayer".to_owned(),
            payload:
                "\"0x031288a6788678c25952eba8693b2f278f66e2187004b64ac09416d07f83f96d5b\""
                    .to_owned(),
        },
    };
    let signed_tx = SignedTransaction {
        raw:       raw_tx,
        tx_hash:   Hash::from_empty(),
        pubkey:    Bytes::from(
            hex::decode("037d7f0255271bb468bdaa46f0dfd5f6130f1c8ea2c1bc016d69df0b4ddee1cc4f")
                .unwrap(),
        ),
        signature: BytesMut::from("").freeze(),
    };
    let txs = vec![signed_tx];
    let executor_resp = executor.exec(&params, &txs).unwrap();
    let receipt = &executor_resp.receipts[0];
    let response = &receipt.response.response;
    let events = &receipt.events;
    assert_eq!(response.is_error(), false);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].topic, "NewRelayerEvent");
}

fn mock_ckb_message() -> String {
    let mint_payload = MintSudt {
        id:       Hash::from_hex(
            "0xf56924db538e77bb5951eb5ff0d02b88983c49c45eea30e8ae3e7234b311436c",
        )
        .unwrap(),
        receiver: Address::from_hex("0x016cbd9ee47a255a6f68882918dcdd9e14e6bee1").unwrap(),
        amount:   100,
    };
    let batch_mint_payload = BatchMintSudt {
        batch: vec![mint_payload.clone(), mint_payload],
    };
    let batch_mint_payload = Bytes::from(serde_json::to_vec(&batch_mint_payload).unwrap());
    let ckb_message_payload = "0x".to_owned() + &hex::encode(batch_mint_payload.clone());
    let payload_hash = Hash::digest(batch_mint_payload);
    let hash_value = HashValue::try_from(payload_hash.as_bytes().as_ref()).unwrap();
    let private_key = Hex::from_string(
        "0x30269d47fcf602b889243722b666881bf953f1213228363d34cf04ddcd51dfd2".to_owned(),
    )
    .unwrap()
    .as_bytes()
    .unwrap();
    let secp_private = Secp256k1PrivateKey::try_from(private_key.as_ref()).unwrap();
    let signature = secp_private.sign_message(&hash_value).to_bytes();
    let signature = "0x".to_owned() + &hex::encode(signature.clone());
    let ckb_message = CKBMessage {
        payload:   Hex::from_string(ckb_message_payload).unwrap(),
        signature: Hex::from_string(signature).unwrap(),
    };
    serde_json::to_string(&ckb_message).unwrap()
}

fn mock_executor_and_params() -> (
    ServiceExecutor<MockStorage, MemoryDB, MockServiceMapping>,
    ExecutorParams,
) {
    let memdb = Arc::new(MemoryDB::new(false));
    let arcs = Arc::new(MockStorage {});
    let toml_str = include_str!("./mock_genesis.toml");
    let genesis: Genesis = toml::from_str(toml_str).unwrap();
    let root = ServiceExecutor::create_genesis(
        genesis.services,
        Arc::clone(&memdb),
        Arc::new(MockStorage {}),
        Arc::new(MockServiceMapping {}),
    )
    .unwrap();
    let executor = ServiceExecutor::with_root(
        root.clone(),
        Arc::clone(&memdb),
        Arc::clone(&arcs),
        Arc::new(MockServiceMapping {}),
    )
    .unwrap();
    let params = ExecutorParams {
        state_root:   root,
        height:       1,
        timestamp:    0,
        cycles_limit: std::u64::MAX,
    };
    (executor, params)
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

    async fn insert_hook_receipt(&self, _receipt: BlockHookReceipt) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn get_hook_receipt(&self, _height: u64) -> ProtocolResult<BlockHookReceipt> {
        unimplemented!()
    }
}

pub struct MockServiceMapping;

impl ServiceMapping for MockServiceMapping {
    fn get_service<SDK: 'static + ServiceSDK>(
        &self,
        name: &str,
        sdk: SDK,
    ) -> ProtocolResult<Box<dyn Service>> {
        let service = match name {
            "ckb_handler" => Box::new(CKBHandler::new(sdk)) as Box<dyn Service>,
            "ckb_sudt" => Box::new(CKBSudt::new(sdk)) as Box<dyn Service>,
            _ => panic!("not found service"),
        };
        Ok(service)
    }

    fn list_service_name(&self) -> Vec<String> {
        vec!["ckb_handler".to_owned(), "ckb_sudt".to_owned()]
    }
}
