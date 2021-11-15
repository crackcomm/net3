//! Common client utilities.

use std::{io::Result, marker::PhantomData};

use async_trait::async_trait;

use crate::{
    builder::ClientHandle,
    handle::Handle,
    traits::{Handler, HandlerBuilder, Initializer},
};

use net3_msg::traits::Message;

/// Default handler builder.
#[derive(Clone)]
pub struct DefaultBuilder<H: Default>(PhantomData<H>);

#[async_trait]
impl<H: Handler + Send + Default> HandlerBuilder for DefaultBuilder<H> {
    type Handler = H;

    #[inline]
    async fn build_handler(&mut self, _: &ClientHandle<Self::Handler>) -> Self::Handler {
        Default::default()
    }
}

impl<H: Default> Default for DefaultBuilder<H> {
    #[inline]
    fn default() -> Self {
        DefaultBuilder(PhantomData)
    }
}

/// Clone handler builder.
#[derive(Clone)]
pub struct CloneBuilder<H: Clone>(pub H);

#[async_trait]
impl<H: Handler + Send + Clone> HandlerBuilder for CloneBuilder<H> {
    type Handler = ClonedHandler<H>;

    #[inline]
    async fn build_handler(&mut self, handle: &ClientHandle<Self::Handler>) -> Self::Handler {
        ClonedHandler {
            handler: self.0.clone(),
            handle: handle.clone(),
        }
    }
}

impl<H: Handler + Send + Clone> From<H> for CloneBuilder<H> {
    #[inline]
    fn from(handler: H) -> Self {
        CloneBuilder(handler)
    }
}

/// Cloned handler.
///
/// It preserves a `ClientHandle<H>` so it doesn't get dropped.
pub struct ClonedHandler<H: Handler> {
    handler: H,
    #[allow(dead_code)]
    handle: ClientHandle<H>,
}

#[async_trait]
impl<H: Handler + Send> Handler for ClonedHandler<H> {
    type Event = H::Event;
    type Message = H::Message;

    /// Handles event message.
    async fn handle_notification(&mut self, message: Self::Message) -> Result<Vec<Self::Message>> {
        self.handler.handle_notification(message).await
    }

    /// Handles request message.
    async fn handle_request(&mut self, message: Self::Message) -> Result<Vec<Self::Message>> {
        self.handler.handle_request(message).await
    }

    /// Handles internal event.
    async fn handle_internal_event(&mut self, event: Self::Event) -> Result<Vec<Self::Message>> {
        self.handler.handle_internal_event(event).await
    }
}

/// Handler builder using `From` conversion.
///
/// Builders for handlers that can build `From<Handle>`.
pub struct FromBuilder<H>(PhantomData<H>);

#[async_trait]
impl<H: Handler + Send> HandlerBuilder for FromBuilder<H>
where
    H: for<'a> From<&'a ClientHandle<H>>,
{
    type Handler = H;

    #[inline]
    async fn build_handler(&mut self, handle: &ClientHandle<H>) -> H {
        H::from(handle)
    }
}

impl<B> Default for FromBuilder<B> {
    #[inline]
    fn default() -> Self {
        FromBuilder(PhantomData)
    }
}

impl<B> Clone for FromBuilder<B> {
    #[inline]
    fn clone(&self) -> Self {
        FromBuilder(PhantomData)
    }
}

/// Take handler builder.
///
/// It should not be generally used!
/// Reconnects with this builder will result in a runtime panic.
pub struct TakeBuilder<H: Handler + Send>(pub Option<H>);

#[async_trait]
impl<H: Handler + Send> HandlerBuilder for TakeBuilder<H> {
    type Handler = H;

    #[inline]
    async fn build_handler(&mut self, _: &ClientHandle<Self::Handler>) -> Self::Handler {
        if let Some(handler) = self.0.take() {
            handler
        } else {
            panic!("Tried to reuse `TakeBuilder`.")
        }
    }
}

impl<H: Handler + Send> From<H> for TakeBuilder<H> {
    #[inline]
    fn from(handler: H) -> Self {
        TakeBuilder(Some(handler))
    }
}

/// Connection initialization function type alias.
pub type InitFunc<M, U = ()> = Box<dyn Fn(&Handle<M, U>) + Sync + Send + 'static>;

/// Handler initialization function type alias.
pub type HandlerInitFunc<H> = InitFunc<<H as Handler>::Message, <H as Handler>::Event>;

/// Handler builder initialization function type alias.
pub type BuilderInitFunc<B> = HandlerInitFunc<<B as HandlerBuilder>::Handler>;

/// Initialization handler builder.
pub struct InitBuilder<B: HandlerBuilder> {
    inner: B,
    init_funcs: Vec<BuilderInitFunc<B>>,
}

impl<B: HandlerBuilder + Send> InitBuilder<B> {
    pub fn add_init(&mut self, init: BuilderInitFunc<B>) {
        self.init_funcs.push(init);
    }
}

#[async_trait]
impl<B: HandlerBuilder + Send> HandlerBuilder for InitBuilder<B> {
    type Handler = <B as HandlerBuilder>::Handler;

    #[inline]
    async fn build_handler(&mut self, handle: &ClientHandle<Self::Handler>) -> Self::Handler {
        for init in &self.init_funcs {
            (*init)(handle);
        }
        self.inner.build_handler(handle).await
    }
}

impl<B: HandlerBuilder + Send> From<B> for InitBuilder<B> {
    fn from(inner: B) -> Self {
        InitBuilder {
            inner,
            init_funcs: vec![],
        }
    }
}

/// Closure initialization helper.
pub struct InitClosure<M: Message, U> {
    inner: InitFunc<M, U>,
}

#[async_trait]
impl<M: Message, U: Send> Initializer<M, U> for InitClosure<M, U> {
    async fn init(&mut self, handle: &Handle<M, U>) -> std::io::Result<()> {
        (*self.inner)(handle);
        Ok(())
    }
}
impl<M: Message, U: Send> From<InitFunc<M, U>> for InitClosure<M, U> {
    fn from(inner: InitFunc<M, U>) -> Self {
        InitClosure { inner }
    }
}

mod noop {
    use std::fmt::Debug;

    use net3_msg::traits::Message;

    use crate::{handle::Handle, traits::Handler};

    /// No-op handler for messages.
    #[derive(Clone)]
    pub struct NoopHandler<M: Message, U = ()>(Option<Handle<M, U>>);

    #[async_trait::async_trait]
    impl<M: Message, U> Handler for NoopHandler<M, U>
    where
        U: Sized + Send + Sync + Clone + Debug,
    {
        type Event = U;
        type Message = M;
    }

    impl<'a, M: Message, U> From<&'a Handle<M, U>> for NoopHandler<M, U> {
        fn from(handle: &'a Handle<M, U>) -> Self {
            NoopHandler(Some(handle.clone()))
        }
    }

    impl<M: Message, U> Default for NoopHandler<M, U> {
        fn default() -> Self {
            NoopHandler(None)
        }
    }
}

pub use self::noop::*;
