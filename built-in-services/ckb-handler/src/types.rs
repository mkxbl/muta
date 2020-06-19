use ckb_jsonrpc_types::Transaction;
use ckb_types::core::TransactionView;
use ckb_types::packed::Transaction as PackedTransaction;
use molecule::prelude::Entity;
use serde::{Deserialize, Serialize};

use protocol::types::{Address, Hash};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CKBMessage {
    pub number: u64,
    pub txs:    Vec<Transaction>,
    pub proof:  MsgProof,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MsgProof {
    pub indices:        Vec<u32>,
    pub lemmas:         Vec<Hash>,
    pub witnesses_root: Hash,
}

pub struct MsgView {
    pub number: u64,
    pub txs:    Vec<TransactionView>,
    pub proof:  MsgProof,
}

impl From<CKBMessage> for MsgView {
    fn from(input: CKBMessage) -> Self {
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
            witnesses_root: self.proof.witnesses_root.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VerifyMsgPayload {
    pub number:         u64,
    pub indices:        Vec<u32>,
    pub lemmas:         Vec<Hash>,
    pub leaves:         Vec<Hash>,
    pub witnesses_root: Hash,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MintSudtsPayload {
    pub id:       Hash,
    pub receiver: Address,
    pub amount:   u128,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SubmitMessageEvent {
    pub number:    u64,
    pub tx_hashes: Vec<Hash>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    #[test]
    fn test_ckb_message_codec() {
        let json = "{\"number\":1, \"txs\":[{\"cell_deps\":[{\"dep_type\":\"code\",\"out_point\":{\"index\":\"0x0\",\"tx_hash\":\"0xa4037a893eb48e18ed4ef61034ce26eba9c585f15c9cee102ae58505565eccc3\"}}],\"header_deps\":[\"0x7978ec7ce5b507cfb52e149e36b1a23f6062ed150503c85bbf825da3599095ed\"],\"inputs\":[{\"previous_output\":{\"index\":\"0x0\",\"tx_hash\":\"0x365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17\"},\"since\":\"0x0\"}],\"outputs\":[{\"capacity\":\"0x2540be400\",\"lock\":{\"args\":\"0x\",\"code_hash\":\"0x28e83a1277d48add8e72fadaa9248559e1b632bab2bd60b27955ebc4c03800a5\",\"hash_type\":\"data\"},\"type\":null}],\"outputs_data\":[\"0x\"],\"version\":\"0x0\",\"witnesses\":[]}], \"proof\":{\"indices\":[1], \"lemmas\":[\"0x365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17\"], \"witnesses_root\": \"0x365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17\"}}";

        let msg: CKBMessage = serde_json::from_str(json);
        assert_eq!(msg.is_ok(), true);
        println!("{:?}", payload);
    }
}
