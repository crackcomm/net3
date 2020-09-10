
use async_trait::async_trait;
use ice_net_message::{Message, Result};

// #[derive(Debug, err_derive::Error)]
// pub enum Error {
//     /// RPC method call error.
//     ///
//     /// Returned as response to the caller.
//     #[error(display = "rpc: {:?}", _0)]
//     Rpc(ice_net_message::Error),
// }

// #[async_trait::async_trait]
// pub trait RequestHandler<M: Message> {
//     async fn handle_rpc_request(&mut self, _message: M) -> Result<Vec<M>>;
// }
