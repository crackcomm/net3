//! JSON-RPC error structures.

use net3_msg::types::ErrorKind;

use super::{ErrorCode, Params};

/// Error object as defined in Spec
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
    /// Code
    pub code: ErrorCode,
    /// Message
    pub message: String,
    /// Optional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Params>,
}

impl Error {
    /// Wraps given `ErrorCode`
    pub fn new(code: ErrorCode) -> Self {
        Error {
            message: code.description().to_string(),
            code,
            data: None,
        }
    }

    /// Creates new `MethodNotFound`
    pub fn method_not_found() -> Self {
        Self::new(ErrorCode(ErrorKind::MethodNotFound))
    }

    /// Creates new `InternalError`
    pub fn internal_error() -> Self {
        Self::new(ErrorCode(ErrorKind::InternalError))
    }

    pub fn into_error(self) -> net3_msg::types::Error {
        net3_msg::types::Error {
            kind: self.code.0,
            description: Some(self.message),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.description(), self.message)
    }
}

impl std::error::Error for Error {}
