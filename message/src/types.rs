use std::{borrow::Cow, fmt};

/// Message kind.
///
/// Default message kind is an `Event`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    Undefined,
    Event,
    Request,
    Response,
    ErrorResponse,
}

impl Default for MessageKind {
    fn default() -> Self {
        MessageKind::Event
    }
}

/// Message unique identifier in the context of a [`Channel`].
///
/// NOTE: Current `TryInto<u64>` implementation returns `Ok(0)` on `Null` id.
///
/// [`Channel`]: ../channel/type.Channel.html
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum Id {
    /// Empty message identifier.
    ///
    /// This is a default identifier of [`Event`] messages.
    ///
    /// [`Event`]: enum.MessageKind.html#variant.Event
    Null,

    /// String identifier of a message.
    Str(String),

    /// Numerical identifier of a message.
    Num(u64),
}

impl Id {
    /// Returns true if `Id` is of a kind `Null`.
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Id::Null)
    }

    /// Returns true if `Id` is not of a kind `Null`.
    #[inline]
    pub fn is_some(&self) -> bool {
        !matches!(self, Id::Null)
    }

    /// Returns identifier as a string.
    #[inline]
    pub fn as_str(&self) -> Option<Cow<'_, str>> {
        match self {
            Id::Null => None,
            Id::Str(id) => Some(Cow::Borrowed(id)),
            Id::Num(id) => Some(Cow::Owned(id.to_string())),
        }
    }
}

impl std::convert::TryFrom<&Id> for u64 {
    type Error = std::num::ParseIntError;

    #[inline]
    fn try_from(value: &Id) -> std::result::Result<Self, Self::Error> {
        match value {
            Id::Null => Ok(0),
            Id::Num(num) => Ok(*num),
            Id::Str(num) => num.parse(),
        }
    }
}

impl fmt::Display for Id {
    /// Formats identifier for display.
    ///
    /// [`Null`] identifier is serialized same as `Str("null")`.
    ///
    /// [`Null`]: enum.Id.html#variant.Null
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::Null => f.write_str("null"),
            Id::Str(id) => write!(f, "{}", id),
            Id::Num(id) => write!(f, "{}", id),
        }
    }
}

impl Default for Id {
    #[inline]
    fn default() -> Self {
        Id::Null
    }
}

/// Error kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    /// Internal server error.
    InternalError,

    /// Method not found error.
    MethodNotFound,

    /// Server error code.
    ErrorCode(i64),
}

impl ErrorKind {
    /// Returns short-string format of error kind.
    #[inline]
    pub fn description(&self) -> Cow<'static, str> {
        match self {
            ErrorKind::InternalError => Cow::Borrowed("internal server error"),
            ErrorKind::MethodNotFound => Cow::Borrowed("method not found"),
            ErrorKind::ErrorCode(code) => Cow::Owned(format!("error code: {}", code)),
        }
    }
}

impl fmt::Display for ErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description().as_ref())
    }
}

/// Message error type.
#[derive(Debug)]
pub struct Error {
    /// Error kind or error code.
    pub kind: ErrorKind,

    /// Optional error description.
    pub description: Option<String>,
}

impl Error {
    /// Creates a new error.
    pub fn new(kind: ErrorKind, description: Option<String>) -> Self {
        Error { kind, description }
    }
}

impl std::error::Error for Error {}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind,
            description: None,
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "code: {} details: {:?}", self.kind, self.description)
    }
}

/// Result type alias with message error type.
pub type Result<T> = std::result::Result<T, Error>;
