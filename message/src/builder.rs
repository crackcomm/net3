use crate::types;

use serde::ser::Serialize;

/// Creates a new message builder.
pub fn new<T: MessageBuilderExt>(kind: types::MessageKind) -> <T as MessageBuilderExt>::Builder {
    <T as MessageBuilderExt>::Builder::new(kind)
}

/// Creates a new event message builder.
pub fn new_empty_event<T: MessageBuilderExt>(method: &str) -> <T as MessageBuilderExt>::Builder {
    <T as MessageBuilderExt>::Builder::new_event(method)
}

/// Creates a new event message builder.
pub fn new_event<T: MessageBuilderExt, V: Serialize>(
    method: &str,
    params: Option<&V>,
) -> std::io::Result<<T as MessageBuilderExt>::Builder> {
    let mut builder = <T as MessageBuilderExt>::Builder::new_event(method);
    if let Some(params) = params {
        builder.set_data(params)?;
    }
    Ok(builder)
}

/// Creates a new request message builder.
pub fn new_request<T: MessageBuilderExt, V: Serialize>(
    id: types::Id,
    method: &str,
    params: Option<&V>,
) -> std::io::Result<<T as MessageBuilderExt>::Builder> {
    let mut builder = <T as MessageBuilderExt>::Builder::new_request(id, method);
    if let Some(params) = params {
        builder.set_data(params)?;
    }
    Ok(builder)
}

/// Creates a new request message builder.
pub fn new_empty_request<T: MessageBuilderExt>(
    id: types::Id,
    method: &str,
) -> <T as MessageBuilderExt>::Builder {
    <T as MessageBuilderExt>::Builder::new_request(id, method)
}

/// Creates a new response message builder.
pub fn new_response<T: MessageBuilderExt>(request: &T) -> <T as MessageBuilderExt>::Builder {
    <T as MessageBuilderExt>::Builder::new_response(request)
}

/// Creates a new error response message builder.
pub fn new_error_response<T: MessageBuilderExt>(
    request: &T,
    error: types::Error,
) -> <T as MessageBuilderExt>::Builder {
    <T as MessageBuilderExt>::Builder::new_error_response(request, error)
}

/// Message builder trait.
pub trait MessageBuilder<M: Sized>: Sized {
    fn new(kind: types::MessageKind) -> Self;

    fn set_id(&mut self, id: types::Id);

    fn set_data<T: Serialize>(&mut self, data: &T) -> std::io::Result<()>;

    fn set_event_name<T: ToString>(&mut self, name: T);

    fn set_method_name<T: ToString>(&mut self, method: T);

    fn with_id(mut self, id: types::Id) -> Self {
        self.set_id(id);
        self
    }

    fn with_data<T: Serialize>(mut self, data: &T) -> std::io::Result<Self> {
        self.set_data(data)?;
        Ok(self)
    }

    fn with_event_name<T: ToString>(mut self, name: T) -> Self {
        self.set_event_name(name);
        self
    }

    fn with_method_name<T: ToString>(mut self, method: T) -> Self {
        self.set_method_name(method);
        self
    }

    fn new_event<T: ToString>(name: T) -> Self {
        let mut msg = Self::new(types::MessageKind::Event);
        msg.set_event_name(name);
        msg
    }

    fn new_request<T: ToString>(id: types::Id, method: T) -> Self {
        let mut msg = Self::new(types::MessageKind::Request);
        msg.set_id(id);
        msg.set_method_name(method);
        msg
    }

    fn new_response(request: &M) -> Self;

    fn new_error_response(request: &M, error: types::Error) -> Self;

    fn build(self) -> M;
}

/// Message builder extension trait.
pub trait MessageBuilderExt: Sized {
    type Builder: MessageBuilder<Self>;
}
