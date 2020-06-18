use std::collections::BTreeMap;

use muta_codec_derive::RlpFixedCodec;
use serde::{Deserialize, Serialize};

use binding_macro::{SchemaEvent, SchemaObject};
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::traits::MetaGenerator;
use protocol::types::{Address, Bytes, DataMeta, FieldMeta, Hash, Hex, StructMeta};
use protocol::ProtocolResult;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct HandlerConfig {
    pub relayer_pubkey: Bytes,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, SchemaObject)]
pub struct CrossMessage {
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
    pub new_relayer: Bytes,
}

#[derive(SchemaEvent)]
pub enum Events {
    NewRelayerEvent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::fixed_codec::{FixedCodec, FixedCodecError};

    #[test]
    fn test_payload() {
        let mint_payload = MintSudt {
            id:       Hash::from_hex(
                "0xf56924db538e77bb5951eb5ff0d02b88983c49c45eea30e8ae3e7234b311436c",
            )
            .unwrap(),
            receiver: Address::from_hex("0xf8389d774afdad8755ef8e629e5a154fddc6325a").unwrap(),
            amount:   100,
        };
        let payload = BatchMintSudt {
            batch: vec![mint_payload.clone(), mint_payload],
        };
        let payload_bytes = payload.encode_fixed().unwrap();
        let payload_hex = "0x".to_owned() + &hex::encode(payload_bytes);
        println!("{}", payload_hex)
    }
}
