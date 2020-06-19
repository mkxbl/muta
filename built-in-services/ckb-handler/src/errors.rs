use protocol::traits::ServiceResponse;

pub(crate) enum ServiceError {
    JsonEncode(String),
    InvalidCKBTx(String),
    CallService((u64, String)),
}

impl ServiceError {
    pub fn to_response<T: std::default::Default>(&self) -> ServiceResponse<T> {
        match self {
            Self::JsonEncode(e) => ServiceResponse::<T>::from_error((101, e.as_str())),
            Self::InvalidCKBTx(e) => ServiceResponse::<T>::from_error((102, e.as_str())),
            Self::CallService((c, e)) => ServiceResponse::<T>::from_error((*c, e.as_str())),
        }
    }
}
