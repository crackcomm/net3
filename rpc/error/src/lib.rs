//! net3 rpc error type

/// Network channel method call request error type.
#[derive(Debug, err_derive::Error)]
pub enum Error {
    /// Standard input/output error.
    ///
    /// Returned from `#[rpc]` handler implementation closes connection.
    /// [`Handle`] returns it when connection was closed.
    ///
    /// [`Handle`]: ../net3_client/handle/struct.Handle.html
    #[error(display = "IO: {}", _0)]
    Io(#[source] std::io::Error),

    /// RPC method call error.
    #[error(display = "RPC: {}", _0)]
    Rpc(#[source] net3_msg::types::Error),
}

impl From<std::io::ErrorKind> for Error {
    fn from(kind: std::io::ErrorKind) -> Error {
        Error::Io(std::io::Error::from(kind))
    }
}

unsafe impl Send for Error {}
unsafe impl Sync for Error {}

/// RPC result type alias.
pub type Result<T> = std::result::Result<T, Error>;
