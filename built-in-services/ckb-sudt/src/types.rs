use std::collections::BTreeMap;

use muta_codec_derive::RlpFixedCodec;
use serde::{Deserialize, Serialize};

use binding_macro::{SchemaEvent, SchemaObject};
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::traits::MetaGenerator;
use protocol::types::{Address, Bytes, DataMeta, FieldMeta, Hash, Hex, StructMeta};
use protocol::ProtocolResult;

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, PartialEq, Default, SchemaObject)]
pub struct Sudt {
    pub id:     Hash,
    pub supply: u128,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct GetSupplyPayload {
    pub id: Hash,
}

#[derive(Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct MintSudt {
    pub id:       Hash,
    pub sender:   Hex,
    pub receiver: Address,
    pub amount:   u128,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct BurnSudtPayload {
    pub id:       Hash,
    pub receiver: Hex,
    pub amount:   u128,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct BurnSudt {
    pub id:       Hash,
    pub sender:   Address,
    pub receiver: Hex,
    pub amount:   u128,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct TransferPayload {
    pub id:     Hash,
    pub to:     Address,
    pub amount: u128,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct TransferEvent {
    pub id:     Hash,
    pub from:   Address,
    pub to:     Address,
    pub amount: u128,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct GetBalancePayload {
    pub id:   Hash,
    pub user: Address,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default, SchemaObject)]
pub struct GetBalanceResponse {
    pub id:      Hash,
    pub user:    Address,
    pub balance: u128,
}

#[derive(SchemaEvent)]
pub enum Events {
    MintSudt,
    BurnSudt,
    TransferEvent,
}
