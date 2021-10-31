//! net3 network message channel tokio implementation

use std::{
    fmt::Debug,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    time::Duration,
};

use tokio::{
    io::Error,
    net::{TcpStream, ToSocketAddrs},
    time::{sleep, timeout},
};
use tokio_util::codec::Framed;

/// Default `connect` timeout.
/// Maybe later it will be configurable.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Channel of message through a [`TcpStream`] transport with custom codec.
/// Implements asynchronous [`Sink`] and [`Stream`] interfaces.
///
/// [`TcpStream`]: https://docs.rs/tokio/0.2/tokio/net/struct.TcpStream.html
/// [`Sink`]: https://docs.rs/futures/0.3/futures/sink/trait.Sink.html
/// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
pub struct Channel<C> {
    inner: Framed<TcpStream, C>,
    peer_addr: SocketAddr,
}

impl<C: Default> Channel<C> {
    /// Creates a new channel from [`TcpStream`].
    ///
    /// [`TcpStream`]: https://docs.rs/tokio/0.2/tokio/net/struct.TcpStream.html
    pub fn new(stream: TcpStream) -> Result<Self, Error> {
        let peer_addr = stream.peer_addr()?;
        let inner = Framed::new(stream, Default::default());
        Ok(Channel { inner, peer_addr })
    }

    /// Connects to a TCP endpoint and creates a message [`Channel`].
    ///
    /// [`Channel`]: type.Channel.html
    #[inline]
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<Channel<C>, Error> {
        // Connect to TCP stream.
        // We set nodelay because messages are small size and latency is priority
        let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await??;
        // Create a channel.
        Channel::new(stream)
    }

    /// Connects to a TCP endpoint and creates a message [`Channel`].
    /// Retries to reconnect on failure indefinetely.
    ///
    /// [`Channel`]: type.Channel.html
    #[inline]
    pub async fn connect_infinite<A: ToSocketAddrs + Debug>(
        addr: A,
        retry_interval: Duration,
    ) -> Channel<C> {
        loop {
            match Self::connect(&addr).await {
                Ok(stream) => return stream,
                Err(err) => {
                    log::trace!("Reconnect {:?} error: {}", addr, err);
                    sleep(retry_interval).await;
                    continue;
                }
            }
        }
    }

    /// Returns channel address.
    pub fn peer_addr(&self) -> &SocketAddr {
        &self.peer_addr
    }
}

impl<C> Deref for Channel<C> {
    type Target = Framed<TcpStream, C>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<C> DerefMut for Channel<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
