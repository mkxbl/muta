use std::collections::BTreeMap;
use std::convert::TryInto;

use ckb_jsonrpc_types::Transaction;
use ckb_pow::{DummyPowEngine, EaglesongBlake2bPowEngine, EaglesongPowEngine, PowEngine};
use ckb_types::core::{HeaderBuilder, TransactionView};
use ckb_types::packed::{Byte32, Transaction as PackedTransaction, Uint128, Uint32, Uint64};
use ckb_types::utilities::MergeByte32;
use merkle_cbt::MerkleProof;
use molecule::prelude::Entity;
use muta_codec_derive::RlpFixedCodec;
use serde::{Deserialize, Serialize};

use binding_macro::{SchemaEvent, SchemaObject};
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::traits::MetaGenerator;
use protocol::types::{Address, Bytes, DataMeta, FieldMeta, Hash, Hex, StructMeta};
use protocol::{ProtocolError, ProtocolResult};

#[derive(Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct SubmitMsgPayload {
    pub inner: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MsgPayload {
    pub number: u64,
    pub txs:    Vec<Transaction>,
    pub proof:  MsgProof,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MsgProof {
    pub indices: Vec<u32>,
    pub lemmas:  Vec<Hash>,
}

pub struct MsgView {
    pub number: u64,
    pub txs:    Vec<TransactionView>,
    pub proof:  MsgProof,
}

impl From<MsgPayload> for MsgView {
    fn from(input: MsgPayload) -> Self {
        let mut tx_views = vec![];
        for tx in input.txs.into_iter() {
            let packed_tx: PackedTransaction = PackedTransaction::from(tx);
            tx_views.push(packed_tx.into_view());
        }
        MsgView {
            number: input.number,
            txs:    tx_views,
            proof:  input.proof,
        }
    }
}

impl MsgView {
    pub fn get_verify_payload(&self) -> VerifyMsgPayload {
        let mut leaves = vec![];
        for tx in self.txs.iter() {
            let tx_hash = Hash::from_bytes(tx.hash().as_bytes()).unwrap();
            leaves.push(tx_hash);
        }

        VerifyMsgPayload {
            number: self.number,
            indices: self.proof.indices.clone(),
            lemmas: self.proof.lemmas.clone(),
            leaves,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MintSudtPayload {
    pub id:       Hash,
    pub sender:   Hex,
    pub receiver: Address,
    pub amount:   u128,
}

#[derive(Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct VerifyMsgPayload {
    pub number:  u64,
    pub indices: Vec<u32>,
    pub lemmas:  Vec<Hash>,
    pub leaves:  Vec<Hash>,
}

#[derive(Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct SubmitMsgEvent {
    pub number:   u64,
    pub tx_hashs: Vec<Hash>,
}

#[derive(SchemaEvent)]
pub enum Events {
    SubmitMsgEvent,
}
