//! JSON-RPC message structure.

use std::io::Result;

use net3_msg::prelude::*;

use super::{Error, ErrorCode, Params, Version};

/// JSON-RPC message structure.
///
/// Designed to handle different stratum implementations.
/// It should not be used directly by client facing APIs.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// Protocol version.
    #[serde(default, rename = "jsonrpc")]
    pub version: Version,

    /// Message unique ID.
    #[serde(default, skip_serializing_if = "Id::is_none")]
    pub id: Id,

    /// A String containing the name of the method to be invoked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// Notification parameters.
    #[serde(skip_serializing_if = "Params::is_none")]
    pub params: Params,

    /// Method call result.
    #[serde(skip_serializing_if = "Params::is_none")]
    pub result: Params,

    /// Method call error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

impl traits::Id for Message {
    #[inline(always)]
    fn id(&self) -> &Id {
        &self.id
    }
}

impl traits::Method for Message {
    #[inline(always)]
    fn method(&self) -> Option<&str> {
        self.method.as_deref()
    }
}

impl traits::Error for Message {
    #[inline(always)]
    fn error_kind(&self) -> Option<&types::ErrorKind> {
        self.error.as_ref().map(|err| &err.code.0)
    }

    #[inline(always)]
    fn description(&self) -> Option<&str> {
        self.error.as_ref().map(|err| err.message.as_str())
    }

    #[inline(always)]
    fn into_error(self) -> Option<types::Error> {
        self.error.map(|err| err.into_error())
    }
}

impl traits::Read for Message {
    fn read_optional<T: DeserializeOwned>(&self) -> Result<Option<T>> {
        let params = if self.params.is_some() {
            self.params.value.as_ref()
        } else if self.result.is_some() {
            self.result.value.as_ref()
        } else {
            None
        };
        match params {
            Some(params) => Ok(Some(serde_json::from_str(params.get()).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, err)
            })?)),
            None => Ok(None),
        }
    }
}

impl traits::Kind for Message {
    /// Returns JSON-RPC message kind according to [`specification`].
    ///
    /// Implementation includes tweaks to accept `method` in [`Response`] message[^1].
    ///
    /// [`specification`]: https://www.jsonrpc.org/specification
    /// [`Response`]: ../../message/enum.MessageKind.html#variant.Response
    /// [^1]: https://github.com/mwcproject/mwc-node/blob/master/doc/stratum.md
    fn kind(&self) -> MessageKind {
        // If there is an `ID` it's either `Request`, `Response` or `Error`.
        // https://www.jsonrpc.org/specification#response
        if self.id.is_some() {
            // If `error` is empty it's either `Request` or `Response`.
            if self.error.is_none() {
                // If message `result` is not empty or `method` is empty,
                // this is a successful `Response` messagem.
                // Otherwise it's a `Request` message.
                if self.result.is_some() || self.method.is_none() {
                    MessageKind::Response
                } else {
                    MessageKind::Request
                }
            } else if self.result.is_none() {
                MessageKind::ErrorResponse
            } else {
                MessageKind::Undefined
            }
        } else {
            // To make sure it's a notification validate existence of `method` field.
            // Params field can be left `None` as `ping` command does use it.
            // https://www.jsonrpc.org/specification#notification
            if is_str_empty(&self.method) {
                MessageKind::Event
            } else {
                MessageKind::Undefined
            }
        }
    }
}

impl MessageBuilder<Message> for Message {
    fn new(_: MessageKind) -> Self {
        Message::default()
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn set_data<T: Serialize>(&mut self, data: &T) -> Result<()> {
        if self.method.is_none() {
            self.result = Params::new(data)?;
        } else {
            self.params = Params::new(data)?;
        }
        Ok(())
    }

    fn set_event_name<T: ToString>(&mut self, name: T) {
        self.id = Id::Null;
        self.method = Some(name.to_string());
    }

    fn set_method_name<T: ToString>(&mut self, method: T) {
        self.method = Some(method.to_string());
    }

    fn new_response(request: &Message) -> Self {
        Message {
            version: Version::V2,
            id: request.id.clone(),
            // Default parameters
            error: None,
            method: None,
            params: Params::empty(),
            result: Params::empty(),
        }
    }

    fn new_error_response(request: &Message, error: types::Error) -> Self {
        let kind = error.kind;
        Message {
            version: Version::V2,
            error: Some(Error {
                message: error
                    .description
                    .unwrap_or_else(|| kind.description().to_string()),
                code: ErrorCode(error.kind),
                data: None,
            }),
            id: request.id.clone(),
            // Default parameters
            method: None,
            result: Params::empty(),
            params: Params::empty(),
        }
    }

    fn build(self) -> Message {
        self
    }
}

impl traits::Message for Message {}

impl MessageBuilderExt for Message {
    type Builder = Self;
}

impl<E> From<Message> for std::result::Result<Vec<Message>, E> {
    #[inline]
    fn from(message: Message) -> Self {
        Ok(vec![message])
    }
}

/// Returns true if given string option is empty.
#[inline]
pub(crate) fn is_str_empty(opt: &Option<String>) -> bool {
    match opt {
        Some(ref inner) => inner.is_empty(),
        None => true,
    }
}
