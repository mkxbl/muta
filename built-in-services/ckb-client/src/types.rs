use std::collections::BTreeMap;
use std::convert::TryInto;

use ckb_pow::{DummyPowEngine, EaglesongBlake2bPowEngine, EaglesongPowEngine, PowEngine};
use ckb_types::core::HeaderBuilder;
use ckb_types::packed::{Byte32, Uint128, Uint32, Uint64};
use ckb_types::utilities::MergeByte32;
use merkle_cbt::MerkleProof;
use molecule::prelude::Entity;
use muta_codec_derive::RlpFixedCodec;
use serde::{Deserialize, Serialize};

use binding_macro::{SchemaEvent, SchemaObject};
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::traits::MetaGenerator;
use protocol::types::{Bytes, DataMeta, FieldMeta, Hash, Hex, StructMeta};
use protocol::{ProtocolError, ProtocolResult};

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug)]
pub struct ClientConfig {
    pub genesis_block_hash:      Hash,
    pub version:                 u32,
    pub pow:                     Pow,
    pub finalized_confirmations: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Pow {
    Dummy,
    Eaglesong,
    EaglesongBlake2b,
}

impl Pow {
    pub fn engine(&self) -> Box<dyn PowEngine> {
        match self {
            Pow::Dummy => Box::new(DummyPowEngine),
            Pow::Eaglesong => Box::new(EaglesongPowEngine),
            Pow::EaglesongBlake2b => Box::new(EaglesongBlake2bPowEngine),
        }
    }
}

impl rlp::Decodable for Pow {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        let value: u8 = rlp.at(0)?.as_val()?;
        match value {
            0 => Ok(Pow::Dummy),
            1 => Ok(Pow::Eaglesong),
            2 => Ok(Pow::EaglesongBlake2b),
            _ => Err(rlp::DecoderError::Custom("pow value should be 0, 1 or 2")),
        }
    }
}

impl rlp::Encodable for Pow {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(1);
        match self {
            Pow::Dummy => s.append(&0u8),
            Pow::Eaglesong => s.append(&1u8),
            Pow::EaglesongBlake2b => s.append(&2u8),
        };
    }
}

impl FixedCodec for Pow {
    fn encode_fixed(&self) -> ProtocolResult<Bytes> {
        Ok(Bytes::from(rlp::encode(self)))
    }

    fn decode_fixed(bytes: Bytes) -> ProtocolResult<Self> {
        Ok(rlp::decode(bytes.as_ref()).map_err(FixedCodecError::from)?)
    }
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct HeadersPayload {
    pub headers: Vec<HeaderPayload>,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct HeaderPayload {
    pub compact_target:    Hex,
    pub version:           Hex,
    pub timestamp:         Hex,
    pub number:            Hex,
    pub epoch:             Hex,
    pub parent_hash:       Hash,
    pub transactions_root: Hash,
    pub proposals_hash:    Hash,
    pub uncles_hash:       Hash,
    pub dao:               Hash,
    pub nonce:             Hex,
}

impl TryInto<HeaderBuilder> for HeaderPayload {
    type Error = ProtocolError;

    fn try_into(self) -> Result<HeaderBuilder, Self::Error> {
        let version = Uint32::new_unchecked(self.version.as_bytes()?);
        let parent_hash = Byte32::new_unchecked(self.parent_hash.as_bytes());
        let timestamp = Uint64::new_unchecked(self.timestamp.as_bytes()?);
        let number = Uint64::new_unchecked(self.number.as_bytes()?);
        let proposals_hash = Byte32::new_unchecked(self.proposals_hash.as_bytes());
        let transactions_root = Byte32::new_unchecked(self.transactions_root.as_bytes());
        let compact_target = Uint32::new_unchecked(self.compact_target.as_bytes()?);
        let uncles_hash = Byte32::new_unchecked(self.uncles_hash.as_bytes());
        let epoch = Uint64::new_unchecked(self.epoch.as_bytes()?);
        let dao = Byte32::new_unchecked(self.dao.as_bytes());
        let nonce = Uint128::new_unchecked(self.nonce.as_bytes()?);

        Ok(HeaderBuilder::default()
            .version(version)
            .parent_hash(parent_hash)
            .timestamp(timestamp)
            .number(number)
            .proposals_hash(proposals_hash)
            .transactions_root(transactions_root)
            .compact_target(compact_target)
            .uncles_hash(uncles_hash)
            .epoch(epoch)
            .dao(dao)
            .nonce(nonce))
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct VerifyTxPayload {
    pub number:  u64,
    pub indices: Vec<u32>,
    pub lemmas:  Vec<Hash>,
    pub leaves:  Vec<Hash>,
}

pub type TxProof = MerkleProof<Byte32, MergeByte32>;

impl VerifyTxPayload {
    pub fn verify(&self, root: &Byte32) -> bool {
        let tx_proof = self.get_tx_proof();
        let leaves: Vec<Byte32> = self
            .leaves
            .iter()
            .map(|v| Byte32::new_unchecked(v.as_bytes()))
            .collect();
        tx_proof.verify(root, leaves.as_slice())
    }

    fn get_tx_proof(&self) -> TxProof {
        TxProof::new(
            self.indices.clone(),
            self.lemmas
                .iter()
                .map(|v| Byte32::new_unchecked(v.as_bytes()))
                .collect(),
        )
    }
}
#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct SubmitHeadersEvent {
    pub start_number: u64,
    pub end_number:   u64,
}

#[derive(SchemaEvent)]
pub enum Events {
    SubmitHeadersEvent,
}
