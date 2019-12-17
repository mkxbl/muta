use std::cell::RefCell;
use std::rc::Rc;

use bytes::Bytes;

use protocol::traits::{ServiceState, StoreObject};
use protocol::types::Hash;
use protocol::ProtocolResult;
use protocol::fixed_codec::FixedCodec;


use crate::store::StoreError;

pub struct DefaultStoreObject<S: ServiceState> {
    state: Rc<RefCell<S>>,
    key:   Hash,
}

impl<S: ServiceState> DefaultStoreObject<S> {
    pub fn new(state: Rc<RefCell<S>>, var_name: &str) -> Self {
        Self {
            state,
            key: Hash::digest(Bytes::from(var_name.to_owned() + "object")),
        }
    }
}

impl<S: ServiceState, O: FixedCodec> StoreObject<O> for DefaultStoreObject<S> {
    fn get(&self) -> ProtocolResult<O> {
        self.state
            .borrow()
            .get(&self.key)?
            .ok_or(StoreError::GetNone.into())
    }

    fn set(&mut self, obj: O
    ) -> ProtocolResult<()> {
        self.state.borrow_mut().insert(self.key.clone(), obj)
    }
}