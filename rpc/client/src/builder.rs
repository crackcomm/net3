//! Network channel client builder utilities.

use std::{
    ops::DerefMut,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
};
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    common::{BuilderInitFunc, CloneBuilder, InitClosure},
    handle::{Handle, InnerHandle},
    handler::{internal::ClientMessage, ClientHandler, ClonedReceiver},
    notifications::{NotificationHandler, Notifications},
    traits::*,
};

use net3_channel::Channel;
use net3_msg::traits::Message;
use net3_rpc_conn::start_loop;

/// Client builder error types.
pub mod errors {
    use std::fmt::Debug;

    /// Client [`Builder`] error kind.
    ///
    /// [`Builder`]: ../../builder/struct.Builder.html
    #[derive(Debug)]
    pub enum ErrorKind {
        /// Channel not set.
        ChannelNotSet,

        /// Handler builder not set.
        HandlerBuilderNotSet,

        /// Reconnect address not set.
        AddressNotSet,
    }

    /// Client [`Builder`] error.
    ///
    /// [`Builder`]: ../../builder/struct.Builder.html
    #[derive(Debug, err_derive::Error)]
    pub enum Error {
        /// Client build error.
        #[error(display = "Build error: {:?}", _0)]
        Build(ErrorKind),

        /// Client loop error.
        #[error(display = "Loop error: {}", _0)]
        Loop(#[source] std::io::Error),
    }
}

/// Client loop error type.
pub type BuilderError = errors::Error;

use self::errors::*;

/// Client handle type alias referenced by [`Handler`].
///
/// [`Handler`]: ../../handler/struct.Handler.html
pub type ClientHandle<H> = Handle<<H as Handler>::Message, <H as Handler>::Event>;

type HandlerInitializer<H> =
    Box<dyn Initializer<<H as Handler>::Message, <H as Handler>::Event> + Send + Sync>;

/// Network channel service client builder based on a [`Channel`] implementation.
/// Provides a way to spawn a connection and create client [`Handle`] for request-reply.
///
/// [`Channel`]: ../channel/struct.Channel.html
/// [`Handle`]: ../handle/struct.Handle.html
pub struct Builder<C: Decoder, B: HandlerBuilder> {
    /// Client ID.
    client_id: Option<u64>,
    /// Network connection channel.
    channel: Option<Channel<C>>,
    /// Sender of messages forwarded to network.
    sender: UnboundedSender<ClientMessage<<<B as HandlerBuilder>::Handler as Handler>::Message>>,
    /// Receiver of messages forwarded to network.
    receiver: ClonedReceiver<ClientMessage<<<B as HandlerBuilder>::Handler as Handler>::Message>>,
    /// Atomic request counter for message ID.
    requests: Arc<AtomicU64>,
    /// Default request timeout set on client handles.
    request_timeout: Duration,
    /// Interval between reconnect retries.
    reconnect_interval: Duration,
    /// Handler builder.
    handler_builder: Option<B>,
    /// Connection initializers.
    initializers: Vec<HandlerInitializer<<B as HandlerBuilder>::Handler>>,
    /// Default target of reconnection.
    reconnect: Option<String>,
    /// Counter of client instances.
    client_handles: Arc<AtomicU64>,
    /// Sender of internal events.
    event_sender: UnboundedSender<<<B as HandlerBuilder>::Handler as Handler>::Event>,
    /// Receiver of internal events.
    event_receiver: ClonedReceiver<<<B as HandlerBuilder>::Handler as Handler>::Event>,
}

impl<C: Decoder, T> Builder<C, CloneBuilder<NotificationHandler<<C as Decoder>::Item, T>>>
where
    <C as Decoder>::Item: Message + Clone,
    T: From<<C as Decoder>::Item> + Send + Sync + Clone,
{
    /// Creates a notification sink handler and sets it by default.
    pub fn notify(&mut self, sender: UnboundedSender<T>) {
        let handler = NotificationHandler::from(sender);
        self.handler_builder = Some(CloneBuilder::from(handler));
    }

    /// Creates a notification sink handler and sets it by default.
    ///
    /// Calling this method twice will make previous receivers useless.
    pub fn notifications(&mut self) -> Notifications<<C as Decoder>::Item, T> {
        let (sender, receiver) = unbounded_channel();
        self.notify(sender);
        let handle = self.handle();
        Notifications { handle, receiver }
    }
}

impl<C: Decoder, B: HandlerBuilder> Builder<C, B> {
    /// Creates a new client builder.
    ///
    /// It is required to set handler builder.
    /// Use `Default::default()` for simple use cases.
    #[inline]
    pub fn new() -> Self {
        let (sender, receiver) = unbounded_channel();
        let (event_sender, event_receiver) = unbounded_channel();
        Builder {
            client_id: None,
            channel: None,
            sender,
            receiver: receiver.into(),
            requests: Default::default(),
            request_timeout: Duration::from_secs(3),
            reconnect_interval: Duration::from_millis(100),
            handler_builder: None,
            initializers: vec![],
            reconnect: None,
            client_handles: Default::default(),
            event_sender,
            event_receiver: event_receiver.into(),
        }
    }

    /// Creates a new network channel service client structure from a [`Channel`].
    ///
    /// [`Channel`]: ../channel/struct.Channel.html
    #[inline]
    pub fn from_channel(channel: Channel<C>) -> Self {
        let (sender, receiver) = unbounded_channel();
        let (event_sender, event_receiver) = unbounded_channel();
        Builder {
            client_id: None,
            channel: Some(channel),
            sender,
            receiver: receiver.into(),
            requests: Default::default(),
            request_timeout: Duration::from_secs(3),
            reconnect_interval: Duration::from_millis(100),
            handler_builder: None,
            initializers: vec![],
            reconnect: None,
            client_handles: Default::default(),
            event_sender,
            event_receiver: event_receiver.into(),
        }
    }

    /// Creates a new network channel service client structure from a [`TcpStream`].
    ///
    /// [`TcpStream`]: https://docs.rs/tokio/0.2/tokio/net/struct.TcpStream.html
    #[inline]
    pub fn from_stream(stream: TcpStream) -> tokio::io::Result<Self>
    where
        C: Default,
        B: Default,
    {
        Builder::default().with_stream(stream)
    }

    /// Creates a builder that creates a connection on its own.
    /// The handler builder has to be set after.
    #[inline]
    pub fn from_addr(addr: &str) -> Self
    where
        C: Default,
        B: Default,
    {
        Self::default().with_reconnect(addr)
    }

    /// Sets the client ID accessible using [`client_id`] method.
    ///
    /// [`client_id`]: ../handle/struct.Handle.html#method.client_id
    #[inline]
    pub fn with_id(mut self, id: u64) -> Self {
        self.client_id = Some(id);
        self
    }

    /// Sets the default client builder channel from [`TcpStream`].
    ///
    /// [`TcpStream`]: https://docs.rs/tokio/0.2/tokio/net/struct.TcpStream.html
    #[inline]
    pub fn with_stream(mut self, stream: TcpStream) -> tokio::io::Result<Self>
    where
        C: Default,
    {
        self.channel = Some(Channel::new(stream)?);
        Ok(self)
    }

    /// Sets the default client builder [`Channel`].
    ///
    /// [`Channel`]: ../channel/struct.Channel.html
    #[inline]
    pub fn with_channel(mut self, channel: Channel<C>) -> Self {
        self.channel = Some(channel);
        self
    }

    /// Sets the default target of reconnection.
    ///
    /// Address is DNS-resolved on every call to [`connect`].
    ///
    /// [`connect`]: ../../channel/fn.connect.html
    #[inline]
    pub fn with_reconnect(mut self, addr: &str) -> Self {
        self.reconnect = Some(addr.to_string());
        self
    }

    /// Sets default timeout on [`request`] call.
    ///
    /// Default request timeout is set to 3 seconds.
    ///
    /// [`request`]: ../handle/struct.Handle.html#method.request
    #[inline]
    pub fn with_call_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets interval between reconnect retries after a failure.
    ///
    /// Default retry interval is set to 100 milliseconds.
    ///
    /// [`request`]: ../handle/struct.Handle.html#method.request
    #[inline]
    pub fn with_reconnect_interval(mut self, interval: Duration) -> Self {
        self.reconnect_interval = interval;
        self
    }

    /// Sets client [`Handler`] builder structure.
    ///
    /// [`Handler`]: ../../handler/trait.Handler.html
    #[inline]
    pub fn with_handler_builder(mut self, builder: B) -> Self {
        self.handler_builder = Some(builder);
        self
    }

    /// Adds client [`Initializer`] builder structure.
    ///
    /// [`Initializer`]: ./traits/trait.Initializer.html
    #[inline]
    pub fn with_init<I>(mut self, init: I) -> Self
    where
        I: Initializer<
                <<B as HandlerBuilder>::Handler as Handler>::Message,
                <<B as HandlerBuilder>::Handler as Handler>::Event,
            > + Send
            + Sync
            + 'static,
    {
        self.initializers.push(Box::new(init));
        self
    }

    /// Adds client [`Initializer`] builder structure from boxed closure.
    ///
    /// [`Initializer`]: ./traits/trait.Initializer.html
    #[inline]
    pub fn with_init_fn<I>(self, init: BuilderInitFunc<B>) -> Self
    where
        <<B as HandlerBuilder>::Handler as Handler>::Message: 'static,
        <<B as HandlerBuilder>::Handler as Handler>::Event: 'static,
    {
        self.with_init(InitClosure::from(init))
    }

    /// Creates a clone-able client [`Handle`].
    /// Use it before spawning a task to send requests and notifications.
    ///
    /// [`Handle`]: ../handle/struct.Handle.html
    #[inline]
    pub fn handle(&self) -> ClientHandle<<B as HandlerBuilder>::Handler> {
        let _ = self.client_handles.fetch_add(1, Ordering::SeqCst);
        Handle {
            inner: Arc::new(InnerHandle {
                client_id: self.client_id,
                events: self.event_sender.clone(),
                sender: self.sender.clone(),
                requests: self.requests.clone(),
                request_timeout: self.request_timeout,
                instances: self.client_handles.clone(),
            }),
            is_owned: true,
        }
    }
}

impl<C: Decoder, B: HandlerBuilder> Builder<C, B> {
    /// Sets client builder [`Handler`] structure.
    ///
    /// Converts the [`Handler`] into a [`HandlerBuilder`]
    /// using `Into<B>`.
    ///
    /// [`Handler`]: ../../handler/trait.Handler.html
    /// [`HandlerBuilder`]: ./traits/struct.HandlerBuilder.html
    #[inline]
    pub fn with_handler<H: Handler + Into<B>>(mut self, handler: H) -> Self
    where
        B: HandlerBuilder<Handler = H>,
    {
        self.handler_builder = Some(handler.into());
        self
    }
}

impl<C, B> Builder<C, B>
where
    C: Default + Send + Sync + 'static,
    B: HandlerBuilder + Send + Sync + 'static,
    C: Decoder<Item = <<B as HandlerBuilder>::Handler as Handler>::Message, Error = std::io::Error>,
    <C as Decoder>::Item: Message + Clone,
    C: Encoder<<<B as HandlerBuilder>::Handler as Handler>::Message, Error = std::io::Error>,
    <B as HandlerBuilder>::Handler: Send + Sync + 'static,
    <<B as HandlerBuilder>::Handler as Handler>::Message: Clone,
{
    /// Spawns client handler loop using [`start`] implementation.
    /// Returns tokio task [`JoinHandle`]. Result error should be handled.
    ///
    /// [`start`]: #method.start
    /// [`JoinHandle`]: https://docs.rs/tokio/0.2/tokio/task/struct.JoinHandle.html
    #[inline]
    pub fn spawn(self) -> JoinHandle<Result<(), BuilderError>> {
        tokio::spawn(async move {
            if let Err(e) = self.start().await {
                log::error!("Spawn error: {:?}", e);
                Err(e)
            } else {
                Ok(())
            }
        })
    }

    /// Spawns client handler loop using [`start_loop`] implementation.
    /// Returns an error on any channel protocol or TCP connection error.
    ///
    /// [`start_loop`]: ../handler/fn.start_loop.html
    #[inline]
    pub async fn start(self) -> Result<(), BuilderError> {
        assert!(
            self.handler_builder.is_some(),
            "Handler builder is required"
        );
        if self.reconnect.is_some() {
            self.start_loop_reconnect().await
        } else {
            self.start_loop().await
        }
    }

    #[inline]
    async fn start_loop(mut self) -> Result<(), BuilderError> {
        let handle = self.handle();
        let mut channel = self
            .channel
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotConnected))?;
        for initializer in self.initializers.iter_mut() {
            initializer.init(&handle).await?;
        }
        let handler = self
            .handler_builder
            .ok_or_else(|| Error::Build(ErrorKind::HandlerBuilderNotSet))?
            .build_handler(&handle)
            .await;
        start_loop(
            channel.deref_mut(),
            ClientHandler::new(
                self.receiver,
                handler,
                handle.into(),
                self.client_handles.clone(),
            ),
            Some(self.event_receiver.clone()),
        )
        .await
        .map_err(Error::Loop)
    }

    #[inline]
    async fn start_loop_reconnect(self) -> Result<(), BuilderError> {
        let handle = self.handle();
        let receiver: ClonedReceiver<
            ClientMessage<<<B as HandlerBuilder>::Handler as Handler>::Message>,
        > = self.receiver;
        let reconnect = self
            .reconnect
            .ok_or_else(|| Error::Build(ErrorKind::AddressNotSet))?;
        let mut builder = self
            .handler_builder
            .ok_or_else(|| Error::Build(ErrorKind::HandlerBuilderNotSet))?;
        let initializers = Arc::new(Mutex::new(self.initializers));
        loop {
            // Check if client handles still exist.
            if self.client_handles.load(Ordering::SeqCst) == 0 {
                log::debug!("No more client handles exist for {:?}", reconnect);
                return Ok(());
            }
            // Connect to TCP stream.
            let mut channel =
                Channel::<C>::connect_infinite(&reconnect, self.reconnect_interval).await;
            let handle_ = handle.clone();
            let initializers_ = initializers.clone();
            tokio::spawn(async move {
                // Initialize connection.
                for initializer in initializers_.lock().await.iter_mut() {
                    if initializer.init(&handle_).await.is_err() {
                        let _ok = handle_.close();
                        break;
                    }
                }
            });
            // Build a new handler.
            let handler = builder.build_handler(&handle).await;
            // Start the client loop.
            match start_loop(
                channel.deref_mut(),
                ClientHandler::new(
                    receiver.clone(),
                    handler,
                    handle.clone().into(),
                    self.client_handles.clone(),
                ),
                Some(self.event_receiver.clone()),
            )
            .await
            {
                Ok(()) => continue,
                Err(err) => log::trace!("Connection error: {:?}, reconnecting.", err),
            }
        }
    }

    /// Spawns connection loop in background and returns client [`Handle`].
    /// Client loop becomes detached and can be closed using handle.
    ///
    /// [`Handle`]: ../handle/struct.Handle.html
    #[inline]
    pub fn background(self) -> ClientHandle<<B as HandlerBuilder>::Handler> {
        let handle = self.handle();
        self.spawn();
        handle
    }
}

impl<C: Decoder, B: HandlerBuilder> From<B> for Builder<C, B> {
    #[inline]
    fn from(handler: B) -> Self {
        let (sender, receiver) = unbounded_channel();
        let (event_sender, event_receiver) = unbounded_channel();
        Builder {
            client_id: None,
            channel: None,
            sender,
            receiver: receiver.into(),
            requests: Default::default(),
            request_timeout: Duration::from_secs(3),
            reconnect_interval: Duration::from_millis(100),
            handler_builder: Some(handler),
            initializers: vec![],
            reconnect: None,
            client_handles: Default::default(),
            event_sender,
            event_receiver: event_receiver.into(),
        }
    }
}

impl<C: Decoder, B: HandlerBuilder> Default for Builder<C, B>
where
    C: Default,
    B: Default,
{
    /// Creates a new network channel service client structure from a [`Channel`].
    ///
    /// [`Channel`]: ../channel/struct.Channel.html
    #[inline]
    fn default() -> Self {
        let (sender, receiver) = unbounded_channel();
        let (event_sender, event_receiver) = unbounded_channel();
        Builder {
            client_id: None,
            channel: None,
            sender,
            receiver: receiver.into(),
            requests: Default::default(),
            request_timeout: Duration::from_secs(3),
            reconnect_interval: Duration::from_millis(100),
            handler_builder: Some(Default::default()),
            initializers: vec![],
            reconnect: None,
            client_handles: Default::default(),
            event_sender,
            event_receiver: event_receiver.into(),
        }
    }
}
