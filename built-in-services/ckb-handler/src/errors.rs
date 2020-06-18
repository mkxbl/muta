use protocol::traits::ServiceResponse;

pub(crate) const PERMISSION_ERROR: (u64, &str) = (101, "wrong permission");

pub(crate) enum ServiceError {
    InvalidMessagePayload(String),
    InvalidMessageSignature(String),
    JsonEncode(String),
    CallService((u64, String)),
}

impl ServiceError {
    pub fn to_response<T: std::default::Default>(&self) -> ServiceResponse<T> {
        match self {
            Self::InvalidMessagePayload(e) => ServiceResponse::<T>::from_error((102, e.as_str())),
            Self::InvalidMessageSignature(e) => ServiceResponse::<T>::from_error((103, e.as_str())),
            Self::JsonEncode(e) => ServiceResponse::<T>::from_error((104, e.as_str())),
            Self::CallService((c, e)) => ServiceResponse::<T>::from_error((*c, e.as_str())),
        }
    }
}
