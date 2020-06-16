pub mod errors;
pub mod types;

use std::collections::BTreeMap;
use std::convert::TryInto;

use bytes::Bytes;
use serde_json;

use ckb_types::core::{HeaderBuilder, HeaderView, TransactionView};
use ckb_types::packed::Header;
use molecule::prelude::Entity;

use binding_macro::{genesis, service, write};
use protocol::emit_event;
use protocol::traits::{
    ExecutorParams, MetaGenerator, ServiceResponse, ServiceSDK, StoreMap, StoreUint64,
};
use protocol::types::{
    Address, DataMeta, Event, Hash, Hex, MethodMeta, Receipt, ServiceContext, ServiceMeta,
};

use crate::errors::{DECODE_MSG_ERROR, MINT_SUDT_PAYLOAD_ERROR, VERIFY_MSG_PAYLOAD_ERROR};
use crate::types::{
    Events, MintSudtPayload, MsgPayload, MsgView, SubmitMsgPayload, VerifyMsgPayload,
};

const HANDLED_MSGS_KEY: &str = "handled_msgs_key";
static ADMISSION_TOKEN: Bytes = Bytes::from_static(b"ckb_handler");

pub struct CKBHandler<SDK> {
    sdk:          SDK,
    handled_msgs: Box<dyn StoreMap<Hash, bool>>,
}

#[service(Events)]
impl<SDK: ServiceSDK> CKBHandler<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let handled_msgs = sdk.alloc_or_recover_map::<Hash, bool>(HANDLED_MSGS_KEY);
        Self { sdk, handled_msgs }
    }

    #[write]
    fn submit_msg(
        &mut self,
        ctx: ServiceContext,
        payload: SubmitMsgPayload,
    ) -> ServiceResponse<()> {
        let msg: Result<MsgPayload, _> = serde_json::from_str(payload.inner.as_str());
        if msg.is_err() {
            return ServiceResponse::<()>::from_error(DECODE_MSG_ERROR);
        }
        let msg_view = MsgView::from(msg.unwrap());
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

    fn get_mint_sudt_payload(&self, tx: &TransactionView) -> ServiceResponse<MintSudtPayload> {
        // TODO verify and extract payload from transaction_view
        let payload = MintSudtPayload {
            id:       Hash::from_empty(),
            sender:   Hex::default(),
            receiver: Address::default(),
            amount:   0,
        };
        ServiceResponse::<MintSudtPayload>::from_succeed(payload)
    }
}
