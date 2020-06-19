use std::collections::BTreeMap;

use muta_codec_derive::RlpFixedCodec;
use serde::{Deserialize, Serialize};

use binding_macro::{SchemaEvent, SchemaObject};
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::traits::MetaGenerator;
use protocol::types::{Address, Bytes, DataMeta, FieldMeta, Hash, Hex, StructMeta};
use protocol::ProtocolResult;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct HandlerGenesis {
    pub relayer_pubkey: Hex,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct CKBMessage {
    pub payload:   Hex,
    pub signature: Hex,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct BatchMintSudt {
    pub batch: Vec<MintSudt>,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct MintSudt {
    pub id:       Hash,
    pub receiver: Address,
    pub amount:   u128,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct NewRelayerEvent {
    pub new_relayer: Hex,
}

#[derive(SchemaEvent)]
pub enum Events {
    NewRelayerEvent,
}
