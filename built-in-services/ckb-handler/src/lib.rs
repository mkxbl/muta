pub mod errors;
pub mod types;

use bytes::Bytes;

use ckb_types::core::TransactionView;

use binding_macro::{service, write};
use protocol::traits::{ExecutorParams, ServiceResponse, ServiceSDK, StoreMap};
use protocol::types::{Address, Event, Hash, Receipt, ServiceContext, ServiceMeta};

use crate::errors::{MINT_SUDT_PAYLOAD_ERROR, VERIFY_MSG_PAYLOAD_ERROR};
use crate::types::{MintSudtPayload, MsgPayload, MsgView};

const HANDLED_MSGS_KEY: &str = "handled_msgs_key";
static ADMISSION_TOKEN: Bytes = Bytes::from_static(b"ckb_handler");

pub struct CKBHandler<SDK> {
    sdk:          SDK,
    handled_msgs: Box<dyn StoreMap<Hash, bool>>,
}

#[service]
impl<SDK: ServiceSDK> CKBHandler<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let handled_msgs = sdk.alloc_or_recover_map::<Hash, bool>(HANDLED_MSGS_KEY);
        Self { sdk, handled_msgs }
    }

    #[write]
    fn submit_msg(&mut self, ctx: ServiceContext, payload: MsgPayload) -> ServiceResponse<()> {
        let msg_view = MsgView::from(payload);
        self.handle_msg(&ctx, &msg_view)
    }

    fn handle_msg(&mut self, ctx: &ServiceContext, msg: &MsgView) -> ServiceResponse<()> {
        let res = self.verify_msg(ctx, msg);
        if !res.is_error() {
            return res;
        }

        self.run_msg(ctx, msg)
    }

    fn verify_msg(&self, ctx: &ServiceContext, msg: &MsgView) -> ServiceResponse<()> {
        let payload = msg.get_verify_payload();
        let payload_json = serde_json::to_string(&payload);
        if payload_json.is_err() {
            return ServiceResponse::<()>::from_error(VERIFY_MSG_PAYLOAD_ERROR);
        }
        let verify_response = self.sdk.read(
            ctx,
            Some(ADMISSION_TOKEN.clone()),
            "ckb_client",
            "verify_tx",
            &payload_json.unwrap(),
        );
        if verify_response.is_error() {
            return ServiceResponse::<()>::from_error((
                verify_response.code,
                verify_response.error_message.as_str(),
            ));
        }
        return ServiceResponse::<()>::from_succeed(());
    }

    fn run_msg(&mut self, ctx: &ServiceContext, msg: &MsgView) -> ServiceResponse<()> {
        for tx in msg.txs.iter() {
            let payload_response = self.get_mint_sudt_payload(tx);
            if payload_response.is_error() {
                return ServiceResponse::<()>::from_error((
                    payload_response.code,
                    payload_response.error_message.as_str(),
                ));
            }
            let payload_json = serde_json::to_string(&payload_response.succeed_data);
            if payload_json.is_err() {
                return ServiceResponse::<()>::from_error(MINT_SUDT_PAYLOAD_ERROR);
            }
            let mint_response = self.sdk.write(
                ctx,
                Some(ADMISSION_TOKEN.clone()),
                "ckb_sudt",
                "mint_sudt",
                &payload_json.unwrap(),
            );
            if mint_response.is_error() {
                return ServiceResponse::<()>::from_error((
                    mint_response.code,
                    mint_response.error_message.as_str(),
                ));
            }
        }
        return ServiceResponse::<()>::from_succeed(());
    }

    fn get_mint_sudt_payload(&self, _tx: &TransactionView) -> ServiceResponse<MintSudtPayload> {
        // TODO verify and extract payload from transaction_view
        // let output = tx.output(1);
        // if output.is_none() {
        //     return ServiceResponse::<MintSudtPayload>::from_error(CKB_TX_ERROR);
        // }
        // let lock_script = output.unwrap().lock();
        // let lock_script_code_hash = lock_script.code_hash();
        // let type_script = output.unwrap().type_();
        let payload = MintSudtPayload {
            id:       Hash::from_empty(),
            receiver: Address::default(),
            amount:   0,
        };
        ServiceResponse::<MintSudtPayload>::from_succeed(payload)
    }
}
