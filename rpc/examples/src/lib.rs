//! example use of rpc derive

use net3_rpc_derive::rpc;
use net3_rpc_error::Result;

#[derive(Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub struct MyMessage {
    pub test: String,
}

/// Example RPC trait.
#[rpc]
pub trait Rpc {
    #[rpc(name = "login")]
    async fn login(&mut self, msg: &MyMessage) -> Result<MyMessage>;

    #[rpc(name = "getjobtemplate")]
    async fn get_work(&mut self, msg: &MyMessage) -> Result<MyMessage>;
}

#[derive(Clone)]
pub struct RpcHandler;

#[async_trait::async_trait]
impl Rpc for RpcHandler {
    async fn login(&mut self, msg: &MyMessage) -> Result<MyMessage> {
        Ok(MyMessage {
            test: format!("very {}", msg.test),
        })
    }

    async fn get_work(&mut self, msg: &MyMessage) -> Result<MyMessage> {
        Ok(MyMessage {
            test: format!("very {}", msg.test),
        })
    }
}
