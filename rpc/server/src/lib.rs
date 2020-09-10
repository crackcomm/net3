//! Network channel server builder tools.
//!
//! Server implementation is using same connection [`Handler`] as the client,
//! making it really a peer to peer implementation of a RPC protocol.
//!
//! The server handler is cloned across all connected clients.
//! Instance of a [`Handler`] should be sendable across threads.
//!
//! # Example
//!
//! ```edition2018,no_run
//! use async_trait::async_trait;
//!
//! use net3_msg::compact::Message;
//! use net3_codec_json_lines::Codec;
//! use net3_rpc_server::{common::CloneBuilder, Handler, ServerBuilder};
//!
//! #[derive(Clone, Debug)]
//! struct MyHandler;
//!
//! #[async_trait]
//! impl Handler for MyHandler {
//!     type Event = ();
//!     type Message = Message;
//!
//!     async fn handle_request(
//!         &mut self,
//!         _message: Self::Message,
//!     ) -> std::io::Result<Vec<Self::Message>> {
//!         Ok(vec![])
//!     }
//! }
//!
//! #[tokio::main]
//! pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//!     let server = ServerBuilder::<Codec<Message>, CloneBuilder<MyHandler>>::from(CloneBuilder(MyHandler))
//!         .bind("127.0.0.1:17653")
//!         .await?;
//!     server.start().await.unwrap();
//!     Ok(())
//! }
//! ```
//!
//! [`Handler`]: ../client/trait.Handler.html

use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    stream::StreamExt,
    sync::Mutex,
    task::JoinHandle,
};
use tokio_util::codec::{Decoder, Encoder};

use net3_msg::traits::Message;
pub use net3_rpc_client::{common, Handler, HandlerBuilder};
use net3_rpc_client::{Builder as ClientBuilder, ClientHandle};

/// Network channel [`Server`] builder utility.
///
/// [`Server`]: struct.Server.html
pub struct ServerBuilder<C, B> {
    builder: B,
    codec: PhantomData<C>,
}

impl<C, B> ServerBuilder<C, B> {
    /// Sets server handler by default using [`CloneBuilder`].
    ///
    /// [`CloneBuilder`]: ../ice_nats_client/common/struct.CloneBuilder.html
    pub fn with_handler<H: Handler>(mut self, handler: H) -> Self
    where
        B: From<H>,
    {
        self.builder = handler.into();
        self
    }

    /// Binds an asynchronous [`TcpListener`] to a set of addresses.
    ///
    /// Returns [`Server`] handle.
    ///
    /// [`Server`]: struct.Server.html
    /// [`TcpListener`]: https://docs.rs/tokio/0.2/tokio/net/struct.TcpListener.html
    pub async fn bind<A: ToSocketAddrs>(self, addr: A) -> Result<Server<C, B>, tokio::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Server {
            listener,
            builder: self.builder,
            codec: PhantomData,
        })
    }
}

impl<C, B: Default> Default for ServerBuilder<C, B> {
    fn default() -> Self {
        ServerBuilder {
            builder: Default::default(),
            codec: PhantomData,
        }
    }
}

impl<C, B: HandlerBuilder> From<B> for ServerBuilder<C, B> {
    fn from(builder: B) -> Self {
        ServerBuilder {
            builder,
            codec: PhantomData,
        }
    }
}

/// Network channel server structure.
///
/// Builds connection handlers using a [`HandlerBuilder`].
///
/// [`HandlerBuilder`]: ../ice_nats_client/trait.HandlerBuilder.html
pub struct Server<C, B> {
    listener: TcpListener,
    builder: B,
    codec: PhantomData<C>,
}

impl<C, B> Server<C, B> {
    /// Creates a server [`Builder`].
    ///
    /// [`Builder`]: struct.Builder.html
    pub fn builder(builder: B) -> ServerBuilder<C, B> {
        ServerBuilder {
            builder,
            codec: PhantomData,
        }
    }
}

impl<C, B> Server<C, B>
where
    C: Default + Send + Sync + 'static,
    B: HandlerBuilder + Send + Sync + 'static,
    C: Decoder<Item = <<B as HandlerBuilder>::Handler as Handler>::Message, Error = std::io::Error>,
    <C as Decoder>::Item: Message + Clone,
    C: Encoder<<<B as HandlerBuilder>::Handler as Handler>::Message, Error = std::io::Error>,
    <B as HandlerBuilder>::Handler: Send + Sync + 'static,
    <<B as HandlerBuilder>::Handler as Handler>::Message: Clone,
{
    /// Spawns server in background by launching [`start`] using [`tokio::spawn`].
    ///
    /// [`start`]: #method.start
    /// [`tokio::spawn`]: https://docs.rs/tokio/0.2/tokio/fn.spawn.html
    pub fn background(self) -> JoinHandle<std::io::Result<()>> {
        tokio::spawn(async move { self.start().await })
    }

    /// Starts accepting connections and handling requests.
    pub async fn start(mut self) -> std::io::Result<()> {
        let mut s = self.listener.incoming();
        let builder = RefBuilder {
            inner: Arc::new(Mutex::new(self.builder)),
        };
        let connected: Arc<AtomicU64> = Default::default();
        let mut connections = 0u64;
        while let Some(socket) = s.try_next().await? {
            let connected = connected.clone();
            let connection = connected.fetch_add(1, Ordering::SeqCst);
            log::trace!(
                "Connection {} accepted from {:?}. Total connected: {}",
                connections,
                socket.peer_addr(),
                connection + 1
            );
            let builder = ClientBuilder::<C, RefBuilder<B>>::new()
                .with_id(connections)
                .with_stream(socket)?
                .with_handler_builder(builder.clone());
            connections += 1;
            tokio::spawn(async move {
                if let Err(err) = builder.start().await {
                    let connection = connected.fetch_sub(1, Ordering::SeqCst);
                    log::debug!(
                        "Connection error: {:?}. Total connected: {}",
                        err,
                        connection - 1
                    );
                }
            });
        }
        Ok(())
    }
}

struct RefBuilder<B: HandlerBuilder> {
    inner: Arc<Mutex<B>>,
}

#[async_trait]
impl<B: HandlerBuilder> HandlerBuilder for RefBuilder<B>
where
    B: Send,
    <B as HandlerBuilder>::Handler: Send,
{
    /// Handler type.
    type Handler = <B as HandlerBuilder>::Handler;

    /// Creates a new handler for client.
    async fn build_handler(&mut self, handle: &ClientHandle<Self::Handler>) -> Self::Handler {
        self.inner.lock().await.build_handler(handle).await
    }
}

impl<B: HandlerBuilder> Clone for RefBuilder<B> {
    fn clone(&self) -> Self {
        RefBuilder {
            inner: self.inner.clone(),
        }
    }
}

unsafe impl<B: HandlerBuilder> Send for RefBuilder<B> {}
unsafe impl<B: HandlerBuilder> Sync for RefBuilder<B> {}
