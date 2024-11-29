use async_nats::jetstream::consumer::pull::{MessagesError, OrderedError};
use async_nats::jetstream::consumer::StreamError;
use async_nats::jetstream::context::{
    CreateStreamError,
    GetStreamError,
    PublishError,
    PublishErrorKind,
};
use async_nats::jetstream::stream::{ConsumerError, PurgeError};

use crate::error::Error;

impl From<PublishError> for Error {
    fn from(value: PublishError) -> Self {
        match value.kind() {
            PublishErrorKind::WrongLastSequence => Self::Conflict,
            _ => Self::Internal(value.into()),
        }
    }
}

macro_rules! impl_from {
    ($error:ty, $variant:ident) => {
        impl From<$error> for crate::error::Error {
            fn from(value: $error) -> Self {
                Self::$variant(value.into())
            }
        }
    };
}

impl_from!(CreateStreamError, Internal);
impl_from!(ConsumerError, Internal);
impl_from!(GetStreamError, Internal);
impl_from!(MessagesError, Internal);
impl_from!(OrderedError, Internal);
impl_from!(PurgeError, Internal);
impl_from!(StreamError, Internal);
