pub mod errors;
pub mod types;

use bytes::Bytes;
use ckb_types::core::TransactionView;

use binding_macro::{service, write};
use protocol::traits::{ExecutorParams, ServiceResponse, ServiceSDK, StoreMap};
use protocol::types::{Event, Hash, Receipt, ServiceContext, ServiceMeta};

use crate::errors::ServiceError;
use crate::types::{CKBMessage, MintSudtsPayload, MsgView, SubmitMessageEvent};

const HANDLED_MSGS_KEY: &str = "handled_msgs_key";
static ADMISSION_TOKEN: Bytes = Bytes::from_static(b"ckb_handler");

pub struct CKBHandler<SDK> {
    sdk:              SDK,
    handled_messages: Box<dyn StoreMap<Hash, bool>>,
}

#[service]
impl<SDK: ServiceSDK> CKBHandler<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let handled_messages = sdk.alloc_or_recover_map::<Hash, bool>(HANDLED_MSGS_KEY);
        Self {
            sdk,
            handled_messages,
        }
    }

    #[write]
    fn submit_message(&mut self, ctx: ServiceContext, msg: CKBMessage) -> ServiceResponse<()> {
        let msg_view = MsgView::from(msg);
        let handled_tx_hashes = match self.handle_message(&ctx, &msg_view) {
            Ok(txs) => txs,
            Err(e) => return e.to_response::<()>(),
        };

        let submit_message_event = SubmitMessageEvent {
            number:    msg_view.number,
            tx_hashes: handled_tx_hashes,
        };
        let submit_message_event = match serde_json::to_string(&submit_message_event)
            .map_err(|e| ServiceError::JsonEncode(format!("{}", e)))
        {
            Ok(event) => event,
            Err(e) => return e.to_response::<()>(),
        };
        ctx.emit_event("SubmitMessageEvent".to_owned(), submit_message_event);
        ServiceResponse::<()>::from_succeed(())
    }

    fn handle_message(
        &mut self,
        ctx: &ServiceContext,
        msg: &MsgView,
    ) -> Result<Vec<Hash>, ServiceError> {
        self.verify_message(ctx, msg)?;
        self.run_message(ctx, msg)
    }

    fn verify_message(&self, ctx: &ServiceContext, msg: &MsgView) -> Result<(), ServiceError> {
        let verify_payload = msg.get_verify_payload();
        let verify_payload = serde_json::to_string(&verify_payload)
            .map_err(|e| ServiceError::JsonEncode(format!("{}", e)))?;

        let verify_response = self.sdk.read(
            ctx,
            Some(ADMISSION_TOKEN.clone()),
            "ckb_client",
            "verify_tx",
            &verify_payload,
        );
        if verify_response.is_error() {
            return Err(ServiceError::CallService((
                verify_response.code,
                verify_response.error_message,
            )));
        }
        Ok(())
    }

    fn run_message(
        &mut self,
        ctx: &ServiceContext,
        msg: &MsgView,
    ) -> Result<Vec<Hash>, ServiceError> {
        let mut tx_hashes = vec![];
        for tx in msg.txs.iter() {
            let mint_sudts_payload = self.get_mint_sudts_payload(tx)?;
            let mint_sudts_payload = serde_json::to_string(&mint_sudts_payload)
                .map_err(|e| ServiceError::JsonEncode(format!("{}", e)))?;

            let mint_sudts_response = self.sdk.write(
                ctx,
                Some(ADMISSION_TOKEN.clone()),
                "ckb_sudt",
                "mint_sudts",
                &mint_sudts_payload,
            );
            if mint_sudts_response.is_error() {
                return Err(ServiceError::CallService((
                    mint_sudts_response.code,
                    mint_sudts_response.error_message,
                )));
            }
            let tx_hash = Hash::from_bytes(tx.hash().raw_data())
                .map_err(|e| ServiceError::InvalidCKBTx(format!("{}", e)))?;
            self.handled_messages.insert(tx_hash.clone(), true);
            tx_hashes.push(tx_hash);
        }
        Ok(tx_hashes)
    }

    fn get_mint_sudts_payload(
        &self,
        _tx: &TransactionView,
    ) -> Result<MintSudtsPayload, ServiceError> {
        // TODO: waiting for ckb script ready,
        // TODO: after ckb script is ready, verify and extract payload from
        // transaction_view

        Ok(MintSudtsPayload::default())
    }
}
