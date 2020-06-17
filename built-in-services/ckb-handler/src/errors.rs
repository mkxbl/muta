use protocol::traits::ServiceResponse;

pub(crate) const DECODE_MSG_ERROR: (u64, &str) = (101, "decode message error");
pub(crate) const VERIFY_MSG_PAYLOAD_ERROR: (u64, &str) =
    (102, "verify_msg_payload json encode error");
pub(crate) const MINT_SUDT_PAYLOAD_ERROR: (u64, &str) =
    (103, "mint_sudt_payload json encode error");
pub(crate) const CKB_TX_ERROR: (u64, &str) = (104, "ckb transaction should contain outputs");

pub(crate) enum ServiceError {}
