//! Common client utilities.

use std::marker::PhantomData;

use async_trait::async_trait;

use crate::{
    builder::ClientHandle,
    traits::{Handler, HandlerBuilder},
};

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
    type Handler = H;

    #[inline]
    async fn build_handler(&mut self, _: &ClientHandle<Self::Handler>) -> Self::Handler {
        self.0.clone()
    }
}

impl<H: Handler + Send + Clone> From<H> for CloneBuilder<H> {
    #[inline]
    fn from(handler: H) -> Self {
        CloneBuilder(handler)
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
