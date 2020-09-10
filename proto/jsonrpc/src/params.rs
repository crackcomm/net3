//! JSON-RPC message parameters type.

use std::str::FromStr;

/// Boxed raw JSON value.
pub type RawValue = Box<serde_json::value::RawValue>;

/// Request parameters
#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Params {
    pub value: Option<RawValue>,
}

impl Params {
    /// Creates `Params` by serializing value to a JSON string.
    #[inline]
    pub fn new<T: serde::ser::Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        Self::from_string(serde_json::to_string(value)?)
    }

    /// Creates `Params` from owned JSON string.
    #[inline]
    pub fn from_string(string: String) -> Result<Self, serde_json::Error> {
        Ok(Params {
            value: Some(serde_json::value::RawValue::from_string(string)?),
        })
    }

    /// Returns true if `value` option is `None`.
    #[inline]
    pub fn is_none(&self) -> bool {
        self.value.is_none()
    }

    /// Returns true if `value` option is `Some`.
    #[inline]
    pub fn is_some(&self) -> bool {
        self.value.is_some()
    }

    /// Creates an empty params. Explicit alias to `Default::default()`.
    #[inline]
    pub fn empty() -> Self {
        Default::default()
    }
}

impl FromStr for Params {
    type Err = serde_json::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s.to_owned())
    }
}

impl PartialEq for Params {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (Some(sval), Some(oval)) => sval.get() == oval.get(),
            (None, None) => true,
            _ => false,
        }
    }
}
