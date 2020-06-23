use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use async_trait::async_trait;
use cita_trie::MemoryDB;

use framework::binding::sdk::{DefalutServiceSDK, DefaultChainQuerier};
use framework::binding::state::{GeneralServiceState, MPTTrie};
use protocol::traits::{NoopDispatcher, Storage};
use protocol::types::{
    Address, Block, BlockHookReceipt, Bytes, Hash, Hex, Proof, Receipt, ServiceContext,
    ServiceContextParams, SignedTransaction,
};
use protocol::ProtocolResult;

use crate::types::{BatchMintSudt, BurnSudtPayload, GetBalancePayload, MintSudt, TransferPayload};
use crate::CKBSudt;

#[test]
fn test_burn_sudt() {
    let caller = mock_muta_address();
    let context = mock_context(caller.clone());
    let mut service = mock_ckb_sudt();
    let sudt_id = mock_sudt_id();
    service.mint_sudts(context.clone(), BatchMintSudt {
        batch: vec![MintSudt {
            id:       sudt_id.clone(),
            receiver: caller.clone(),
            amount:   200,
        }],
    });
    service.burn_sudt(context.clone(), BurnSudtPayload {
        id:       sudt_id.clone(),
        receiver: mock_ckb_address(),
        amount:   100,
    });
    let balance = service
        .get_balance(context.clone(), GetBalancePayload {
            id:   sudt_id,
            user: caller,
        })
        .succeed_data
        .balance;
    assert_eq!(balance, 100);
}

#[test]
fn test_mint_sudts() {
    let caller = mock_muta_address();
    let context = mock_context(caller.clone());
    let sudt_id = mock_sudt_id();
    let mut service = mock_ckb_sudt();
    service.mint_sudts(context.clone(), BatchMintSudt {
        batch: vec![MintSudt {
            id:       sudt_id.clone(),
            receiver: caller.clone(),
            amount:   100,
        }],
    });
    let balance = service
        .get_balance(context.clone(), GetBalancePayload {
            id:   sudt_id,
            user: caller,
        })
        .succeed_data
        .balance;
    assert_eq!(balance, 100);
}

#[test]
fn test_transfer_sudt() {
    let caller = mock_muta_address();
    let context = mock_context(caller.clone());
    let sudt_id = mock_sudt_id();
    let mut service = mock_ckb_sudt();
    service.mint_sudts(context.clone(), BatchMintSudt {
        batch: vec![MintSudt {
            id:       sudt_id.clone(),
            receiver: caller.clone(),
            amount:   200,
        }],
    });
    let receiver = Address::from_hex("0x016cbd9ee47a255a6f68882918dcdd9e14e6bee1").unwrap();
    service.transfer(context.clone(), TransferPayload {
        id:     sudt_id.clone(),
        to:     receiver.clone(),
        amount: 100,
    });
    let balance = service
        .get_balance(context.clone(), GetBalancePayload {
            id:   sudt_id,
            user: receiver,
        })
        .succeed_data
        .balance;
    assert_eq!(balance, 100);
}

fn mock_sudt_id() -> Hash {
    Hash::from_hex("0xb6a4d7da21443f5e816e8700eea87610e6d769657d6b8ec73028457bf2ca4036").unwrap()
}

fn mock_muta_address() -> Address {
    Address::from_hex("0x755cdba6ae4f479f7164792b318b2a06c759833b").unwrap()
}

fn mock_ckb_address() -> Hex {
    Hex::from_string("0xc4b123456789".to_owned()).unwrap()
}

fn mock_ckb_sudt() -> CKBSudt<
    DefalutServiceSDK<
        GeneralServiceState<MemoryDB>,
        DefaultChainQuerier<MockStorage>,
        NoopDispatcher,
    >,
> {
    let chain_db = DefaultChainQuerier::new(Arc::new(MockStorage {}));
    let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
    let state = GeneralServiceState::new(trie);
    let sdk = DefalutServiceSDK::new(
        Rc::new(RefCell::new(state)),
        Rc::new(chain_db),
        NoopDispatcher {},
    );
    CKBSudt::new(sdk)
}

fn mock_context(caller: Address) -> ServiceContext {
    let params = ServiceContextParams {
        tx_hash: None,
        nonce: None,
        cycles_limit: 1024 * 1024 * 1024,
        cycles_price: 1,
        cycles_used: Rc::new(RefCell::new(0)),
        caller,
        height: 1,
        timestamp: 0,
        service_name: "service_name".to_owned(),
        service_method: "service_method".to_owned(),
        service_payload: "service_payload".to_owned(),
        extra: Some(Bytes::from_static(b"access_token")),
        events: Rc::new(RefCell::new(vec![])),
    };
    ServiceContext::new(params)
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
