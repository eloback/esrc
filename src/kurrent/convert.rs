use crate::error::Error;

impl From<kurrentdb::Error> for Error {
    fn from(value: kurrentdb::Error) -> Self {
        match value {
            kurrentdb::Error::WrongExpectedVersion { .. } => Self::Conflict,
            _ => Self::Internal(value.into()),
        }
    }
}
