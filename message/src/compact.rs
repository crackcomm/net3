use crate::{builder, traits, types};

use serde::ser::Serialize;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    kind: types::MessageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<types::Error>,
    #[serde(default, skip_serializing_if = "types::Id::is_none")]
    id: types::Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<String>,
}

impl traits::Id for Message {
    fn id(&self) -> &types::Id {
        &self.id
    }
}

impl traits::Method for Message {
    fn method(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl traits::Kind for Message {
    fn kind(&self) -> types::MessageKind {
        self.kind
    }
}

impl traits::Read for Message {
    fn read_optional<T: serde::de::DeserializeOwned>(&self) -> std::io::Result<Option<T>> {
        match self.data.as_deref() {
            Some(data) => {
                Ok(Some(::serde_json::from_str(data).map_err(|_| {
                    std::io::Error::from(std::io::ErrorKind::InvalidData)
                })?))
            }
            None => Ok(None),
        }
    }
}

impl traits::Error for Message {
    fn error_kind(&self) -> Option<&types::ErrorKind> {
        self.error.as_ref().map(|err| &err.kind)
    }

    fn description(&self) -> Option<&str> {
        self.error
            .as_ref()
            .and_then(|err| err.description.as_deref())
    }

    fn into_error(self) -> Option<types::Error> {
        self.error
    }
}

impl traits::Message for Message {}

impl builder::MessageBuilderExt for Message {
    type Builder = Message;
}

impl builder::MessageBuilder<Message> for Message {
    fn new(kind: types::MessageKind) -> Self {
        Message {
            kind,
            id: types::Id::Null,
            name: None,
            data: None,
            error: None,
        }
    }

    fn new_response(request: &Message) -> Self {
        let mut msg = Self::new(types::MessageKind::Response);
        msg.id = request.id.clone();
        // Some protocols want this:
        // msg.name = request.name.clone();
        msg
    }

    fn new_error_response(request: &Message, error: types::Error) -> Self {
        let mut msg = Self::new(types::MessageKind::ErrorResponse);
        msg.id = request.id.clone();
        // Some protocols want this:
        // msg.name = request.name.clone();
        msg.error = Some(error);
        msg
    }

    fn set_id(&mut self, id: types::Id) {
        self.id = id;
    }

    fn set_event_name<T: ToString>(&mut self, name: T) {
        self.name = Some(name.to_string());
    }

    fn set_method_name<T: ToString>(&mut self, method: T) {
        self.name = Some(method.to_string());
    }

    fn set_data<T: Serialize>(&mut self, data: &T) -> std::io::Result<()> {
        self.data = Some(
            ::serde_json::to_string(data)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?,
        );
        Ok(())
    }

    fn build(self) -> Message {
        Message {
            kind: self.kind,
            id: self.id,
            name: self.name,
            error: self.error,
            data: self.data,
        }
    }
}
