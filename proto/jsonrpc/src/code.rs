//! JSON-RPC error codes.

use std::borrow::Cow;

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

use net3_msg::types::ErrorKind;

/// JSON-RPC error code.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ErrorCode(pub ErrorKind);

impl From<ErrorCode> for ErrorKind {
    fn from(code: ErrorCode) -> Self {
        code.0
    }
}

impl From<ErrorKind> for ErrorCode {
    fn from(kind: ErrorKind) -> Self {
        ErrorCode(kind)
    }
}

impl ErrorCode {
    /// Returns short-string format of error kind.
    #[inline]
    pub fn description(&self) -> Cow<'static, str> {
        self.0.description()
    }

    /// Returns integer code value
    pub fn code(&self) -> i64 {
        match self.0 {
            ErrorKind::MethodNotFound => -32601,
            ErrorKind::InternalError => -32603,
            ErrorKind::ErrorCode(code) => code,
        }
    }
}

impl From<i64> for ErrorCode {
    fn from(code: i64) -> Self {
        match code {
            -32601 => ErrorCode(ErrorKind::MethodNotFound),
            -32603 => ErrorCode(ErrorKind::InternalError),
            code => ErrorCode(ErrorKind::ErrorCode(code)),
        }
    }
}

impl<'a> Deserialize<'a> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<ErrorCode, D::Error>
    where
        D: Deserializer<'a>,
    {
        let code: i64 = Deserialize::deserialize(deserializer)?;
        Ok(ErrorCode::from(code))
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.code())
    }
}
