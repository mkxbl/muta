pub mod errors;
pub mod types;

use bytes::Bytes;
use std::collections::BTreeMap;
use std::convert::TryInto;

use ckb_types::core::{HeaderBuilder, HeaderView};
use ckb_types::packed::Header;
use molecule::prelude::Entity;

use binding_macro::{genesis, service, write};
use protocol::emit_event;
use protocol::traits::{
    ExecutorParams, MetaGenerator, ServiceResponse, ServiceSDK, StoreMap, StoreUint64,
};
use protocol::types::{DataMeta, Event, MethodMeta, Receipt, ServiceContext, ServiceMeta};

use crate::errors::{DECODE_HEADER_ERROR, VERIFY_HEADER_FAILED};
use crate::types::{Consensus, HeaderPayload, HeadersPayload, SubmitHeadersEvent, VerifyTxPayload};

const CONSENSUS_KEY: &str = "ckb_consensus_key";
const TIP_HASH_KEY: &str = "tip_hash_key";
const TIP_NUMBER_KEY: &str = "tip_number_key";
const HEADERS_KEY: &str = "ckb_headers_key";

pub struct ClientService<SDK> {
    sdk:        SDK,
    tip_number: Box<dyn StoreUint64>,
    headers:    Box<dyn StoreMap<u64, Bytes>>,
}

#[service]
impl<SDK: ServiceSDK> ClientService<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let tip_number = sdk.alloc_or_recover_uint64(TIP_NUMBER_KEY);
        let headers = sdk.alloc_or_recover_map::<u64, Bytes>(HEADERS_KEY);
        Self {
            sdk,
            tip_number,
            headers,
        }
    }

    #[genesis]
    fn init_genesis(&mut self, payload: Consensus) {
        self.sdk
            .set_value(CONSENSUS_KEY.to_owned(), payload.clone());
        self.sdk.set_value(
            TIP_HASH_KEY.to_owned(),
            payload.genesis_block_hash.as_bytes(),
        );
        self.tip_number.set(0)
    }

    #[write]
    fn submit_headers(
        &mut self,
        ctx: ServiceContext,
        payload: HeadersPayload,
    ) -> ServiceResponse<()> {
        let start_number = self.tip_number.get() + 1;
        for h in payload.headers.into_iter() {
            let header_view: HeaderView =
                match <HeaderPayload as TryInto<HeaderBuilder>>::try_into(h) {
                    Ok(v) => v.build(),
                    Err(_) => return ServiceResponse::<()>::from_error(DECODE_HEADER_ERROR),
                };
            if !self.verify_header(&header_view) {
                return ServiceResponse::<()>::from_error(VERIFY_HEADER_FAILED);
            }
            let number = header_view.number();
            self.headers.insert(number, header_view.data().as_bytes());
            self.tip_number.set(number);
            self.set_tip_hash(header_view.hash().raw_data());
        }
        let end_number = self.tip_number.get();
        emit_event!(ctx, SubmitHeadersEvent {
            start_number,
            end_number
        });
        ServiceResponse::<()>::from_succeed(())
    }

    #[write]
    fn verify_tx(
        &mut self,
        _ctx: ServiceContext,
        payload: VerifyTxPayload,
    ) -> ServiceResponse<bool> {
        let header = match self.headers.get(&payload.number) {
            Some(h) => h,
            None => return ServiceResponse::<bool>::from_succeed(false),
        };
        let root = Header::new_unchecked(header)
            .as_advanced_builder()
            .build()
            .transactions_root();

        ServiceResponse::<bool>::from_succeed(payload.verify(&root))
    }

    fn verify_header(&self, header: &HeaderView) -> bool {
        let consensus = self
            .sdk
            .get_value::<String, Consensus>(&CONSENSUS_KEY.to_owned())
            .expect("consensus should not be none");

        // TODO: verify timestamp and compact_target ?
        if consensus.version != header.version()
            || !consensus.pow.engine().verify(&header.data())
            || self.tip_number.get() + 1 != header.number()
            || self.tip_hash() != header.parent_hash().raw_data()
        {
            return false;
        }

        true
    }

    fn set_tip_hash(&mut self, hash: Bytes) {
        self.sdk.set_value(TIP_HASH_KEY.to_owned(), hash)
    }

    fn tip_hash(&self) -> Bytes {
        self.sdk
            .get_value(&TIP_HASH_KEY.to_owned())
            .expect("tip hash should never be none")
    }
}
