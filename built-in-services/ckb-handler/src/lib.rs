pub mod errors;
pub mod types;

use bytes::Bytes;
use std::collections::BTreeMap;

use binding_macro::{genesis, read, service, write};
use common_crypto::{Crypto, Secp256k1};
use protocol::emit_event;
use protocol::traits::MetaGenerator;
use protocol::traits::{ExecutorParams, ServiceResponse, ServiceSDK};
use protocol::types::{
    Address, DataMeta, Event, Hash, Hex, MethodMeta, Receipt, ServiceContext, ServiceMeta,
};

use crate::errors::{ServiceError, PERMISSION_ERROR};
use crate::types::{BatchMintSudt, CKBMessage, Events, HandlerGenesis, NewRelayerEvent};

const RELAYER_PUBKEY_KEY: &str = "relayer_pubkey_key";
const RELAYER_ADDRESS_KEY: &str = "relayer_address_key";
static ADMISSION_TOKEN: Bytes = Bytes::from_static(b"ckb_handler");

pub struct CKBHandler<SDK> {
    sdk: SDK,
}

#[service(Events)]
impl<SDK: ServiceSDK> CKBHandler<SDK> {
    pub fn new(sdk: SDK) -> Self {
        Self { sdk }
    }

    #[genesis]
    fn init_genesis(&mut self, genesis: HandlerGenesis) {
        self.sdk
            .set_value(RELAYER_PUBKEY_KEY.to_owned(), genesis.relayer_pubkey)
    }

    #[write]
    fn set_relayer(&mut self, ctx: ServiceContext, new_relayer: Hex) -> ServiceResponse<()> {
        let relayer: Hex = self
            .sdk
            .get_value(&RELAYER_PUBKEY_KEY.to_owned())
            .expect("relayer address should never be none");
        let relayer = relayer
            .as_bytes()
            .expect("relayer pubkey hex should never be invalid");
        let relayer =
            Address::from_pubkey_bytes(relayer).expect("relayer address should never be invalid");

        if relayer != ctx.get_caller() {
            return ServiceResponse::<()>::from_error(PERMISSION_ERROR);
        }
        self.sdk
            .set_value(RELAYER_ADDRESS_KEY.to_owned(), new_relayer.clone());

        let new_relayer_event = NewRelayerEvent { new_relayer };
        emit_event!(ctx, new_relayer_event);
        ServiceResponse::<()>::from_succeed(())
    }

    #[read]
    fn get_relayer(&self, _ctx: ServiceContext) -> ServiceResponse<Hex> {
        let relayer: Hex = self
            .sdk
            .get_value(&RELAYER_PUBKEY_KEY.to_owned())
            .expect("relayer pubkey should never be none");
        ServiceResponse::<Hex>::from_succeed(relayer)
    }

    #[write]
    fn submit_message(&mut self, ctx: ServiceContext, msg: CKBMessage) -> ServiceResponse<()> {
        if let Err(e) = self.verify_message(&msg) {
            return e.to_response::<()>();
        }
        if let Err(e) = self.run_message(&ctx, &msg.payload) {
            return e.to_response::<()>();
        }
        ServiceResponse::<()>::from_succeed(())
    }

    fn verify_message(&self, msg: &CKBMessage) -> Result<(), ServiceError> {
        let payload = msg
            .payload
            .as_bytes()
            .map_err(|e| ServiceError::InvalidMessagePayload(format!("{}", e)))?;
        let payload_hash = Hash::digest(payload);
        let signature = msg
            .signature
            .as_bytes()
            .map_err(|e| ServiceError::InvalidMessageSignature(format!("{}", e)))?;
        let pubkey: Hex = self
            .sdk
            .get_value(&RELAYER_PUBKEY_KEY.to_owned())
            .expect("relayer pubkey should never be none");
        let pubkey = pubkey
            .as_bytes()
            .expect("relayer pubkey hex should never be invalid");

        Secp256k1::verify_signature(
            payload_hash.as_bytes().as_ref(),
            signature.as_ref(),
            pubkey.as_ref(),
        )
        .map_err(|e| ServiceError::InvalidMessageSignature(format!("{}", e)))
    }

    fn run_message(&mut self, ctx: &ServiceContext, msg: &Hex) -> Result<(), ServiceError> {
        let payload = msg
            .as_bytes()
            .map_err(|e| ServiceError::InvalidMessagePayload(format!("{}", e)))?;
        let payload: BatchMintSudt = rlp::decode(payload.as_ref())
            .map_err(|e| ServiceError::InvalidMessagePayload(format!("{}", e)))?;

        let payload_json = serde_json::to_string(&payload)
            .map_err(|e| ServiceError::JsonEncode(format!("{}", e)))?;
        let res = self.sdk.write(
            &ctx,
            Some(ADMISSION_TOKEN.clone()),
            "ckb_sudt",
            "mint_sudts",
            &payload_json,
        );
        if res.is_error() {
            return Err(ServiceError::CallService((res.code, res.error_message)));
        }

        Ok(())
    }
}
