pub mod errors;
pub mod types;

use std::collections::BTreeMap;

use binding_macro::{cycles, hook_after, service, write};
use protocol::emit_event;
use protocol::traits::{ExecutorParams, MetaGenerator, ServiceResponse, ServiceSDK, StoreMap};
use protocol::types::{
    Address, DataMeta, Event, Hash, MethodMeta, Receipt, ServiceContext, ServiceMeta,
};

use crate::errors::*;
use crate::types::{
    BurnSudt, BurnSudtPayload, Events, GetBalancePayload, GetBalanceResponse, GetSupplyPayload,
    MintSudt, Sudt, TransferEvent, TransferPayload,
};

const SUDTS_KEY: &str = "sudts";

pub struct CKBSudt<SDK> {
    sdk:   SDK,
    sudts: Box<dyn StoreMap<Hash, Sudt>>,
}

#[service(Events)]
impl<SDK: ServiceSDK> CKBSudt<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let sudts: Box<dyn StoreMap<Hash, Sudt>> = sdk.alloc_or_recover_map(SUDTS_KEY);
        Self { sdk, sudts }
    }

    #[write]
    fn mint_sudt(&mut self, ctx: ServiceContext, payload: MintSudt) -> ServiceResponse<()> {
        if ctx.get_extra().is_none() {
            return ServiceResponse::<()>::from_error(PERMISSION_ERROR);
        }

        let MintSudt {
            id,
            amount,
            receiver,
        } = payload.clone();

        if !self.sudts.contains(&id) {
            let sudt = Sudt {
                id:     id.clone(),
                supply: amount,
            };
            self.sudts.insert(id.clone(), sudt);
            self.sdk.set_account_value(&receiver, id.clone(), amount);
        } else {
            let mut receiver_balance: u128 =
                self.sdk.get_account_value(&receiver, &id).unwrap_or(0);

            let (v, overflow) = receiver_balance.overflowing_add(amount);
            if overflow {
                return ServiceResponse::<()>::from_error(ADD_OVERFLOW);
            }
            receiver_balance = v;
            self.sdk.set_account_value(&receiver, id, receiver_balance);
        }
        emit_event!(ctx, payload);
        ServiceResponse::<()>::from_succeed(())
    }

    #[write]
    fn burn_sudt(&mut self, ctx: ServiceContext, payload: BurnSudtPayload) -> ServiceResponse<()> {
        let sender = ctx.get_caller();
        let BurnSudtPayload {
            id,
            receiver,
            amount,
        } = payload;
        if !self.sudts.contains(&id) {
            return ServiceResponse::<()>::from_error(SUDT_NOT_EXISTED);
        }

        let mut sender_balance: u128 = self.sdk.get_account_value(&sender, &id).unwrap_or(0);

        if sender_balance < amount {
            return ServiceResponse::<()>::from_error(INSUFFICIENT_FUNDS);
        }

        sender_balance -= amount;
        self.sdk
            .set_account_value(&sender, id.clone(), sender_balance);

        emit_event!(ctx, BurnSudt {
            id: id.clone(),
            sender: sender.clone(),
            receiver: receiver.clone(),
            amount,
        });
        ServiceResponse::<()>::from_succeed(())
    }

    #[cycles(210_00)]
    #[write]
    fn transfer(&mut self, ctx: ServiceContext, payload: TransferPayload) -> ServiceResponse<()> {
        let caller = ctx.get_caller();
        let TransferPayload { id, amount, to } = payload;

        if !self.sudts.contains(&id) {
            return ServiceResponse::<()>::from_error(SUDT_NOT_EXISTED);
        }

        if let Err(e) = self._transfer(caller.clone(), to.clone(), id.clone(), amount) {
            return ServiceResponse::<()>::from_error(e);
        };

        let event = TransferEvent {
            id,
            from: caller,
            to,
            amount,
        };
        emit_event!(ctx, event);

        ServiceResponse::<()>::from_succeed(())
    }

    #[cycles(100_00)]
    #[read]
    fn get_supply(&self, ctx: ServiceContext, payload: GetSupplyPayload) -> ServiceResponse<u128> {
        if let Some(sudt) = self.sudts.get(&payload.id) {
            ServiceResponse::<u128>::from_succeed(sudt.supply)
        } else {
            ServiceResponse::<u128>::from_error(SUDT_NOT_EXISTED)
        }
    }

    #[cycles(100_00)]
    #[read]
    fn get_balance(
        &self,
        ctx: ServiceContext,
        payload: GetBalancePayload,
    ) -> ServiceResponse<GetBalanceResponse> {
        let GetBalancePayload { id, user } = payload;
        if !self.sudts.contains(&id) {
            return ServiceResponse::<GetBalanceResponse>::from_error(SUDT_NOT_EXISTED);
        }
        let balance = self.sdk.get_account_value(&user, &id).unwrap_or(0);
        let res = GetBalanceResponse { id, user, balance };
        ServiceResponse::<GetBalanceResponse>::from_succeed(res)
    }

    #[hook_after]
    fn gen_burn_sudt_proof(
        &mut self,
        _params: &ExecutorParams,
        receipts: &[Receipt],
    ) -> Option<Vec<Event>> {
        let mut events = Vec::<Event>::new();
        for r in receipts.iter() {
            for e in r.events.iter() {
                if "ckb_sudt" == e.service.as_str() && "BurnSudt" == e.topic.as_str() {
                    // TODO: aggregate the event of same id, sender, receiver
                    events.push(e.clone());
                }
            }
        }
        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    fn _transfer(
        &mut self,
        sender: Address,
        recipient: Address,
        id: Hash,
        amount: u128,
    ) -> Result<(), (u64, &str)> {
        if recipient == sender {
            return Err(SEND_TO_SELF);
        }

        let mut sender_balance: u128 = self.sdk.get_account_value(&sender, &id).unwrap_or(0);
        if sender_balance < amount {
            return Err(INSUFFICIENT_FUNDS);
        }

        let mut recipient_balance: u128 = self.sdk.get_account_value(&recipient, &id).unwrap_or(0);
        let (v, overflow) = recipient_balance.overflowing_add(amount);
        if overflow {
            return Err(ADD_OVERFLOW);
        }
        recipient_balance = v;
        self.sdk
            .set_account_value(&recipient, id.clone(), recipient_balance);

        let (v, overflow) = sender_balance.overflowing_sub(amount);
        if overflow {
            return Err(ADD_OVERFLOW);
        }
        sender_balance = v;
        self.sdk.set_account_value(&sender, id, sender_balance);

        Ok(())
    }
}
