pub(crate) const DECODE_HEADER_ERROR: (u64, &str) = (101, "header decode error");
pub(crate) const VERIFY_HEADER_FAILED: (u64, &str) = (102, "verify header failed");
pub(crate) const BLOCK_NOT_FINALIZED: (u64, &str) = (103, "the block is not finalized");
pub(crate) const SUBMITTED_BLOCK_NUMBER_ERROR: (u64, &str) = (
    104,
    "submitted block number is greater than tip number in client",
);
pub(crate) const TX_PROOF_ERROR: (u64, &str) = (105, "tx merkle proof verify failed");
