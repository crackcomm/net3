use net3_msg::compact::Message;
use net3_rpc_client::Handle;
use net3_rpc_examples::{Rpc, RpcHandler};
use net3_rpc_server::{Handler, HandlerBuilder};

type Codec = net3_codec_json_lines::Codec<Message>;
type ServerBuilder = net3_rpc_server::ServerBuilder<Codec, MyHandlerBuilder>;
// type ClientBuilder = net3_rpc_client::ClientBuilder<Codec, FromBuilder<NoopHandler<Message>>>;

#[derive(Clone)]
struct MyHandlerBuilder {
    inner: RpcHandler,
}

#[async_trait::async_trait]
impl HandlerBuilder for MyHandlerBuilder {
    type Handler = MyHandler;

    async fn build_handler(&mut self, handle: &Handle<Message, String>) -> Self::Handler {
        MyHandler {
            inner: self.inner.clone(),
            handle: handle.clone(),
        }
    }
}

#[derive(Clone)]
struct MyHandler {
    inner: RpcHandler,
    handle: Handle<Message, String>,
}

#[async_trait::async_trait]
impl Handler for MyHandler {
    type Event = String;
    type Message = Message;

    async fn handle_request(
        &mut self,
        request: Self::Message,
    ) -> std::io::Result<Vec<Self::Message>> {
        println!("Request: {:?}", request);
        match self.inner.handle_message(request).await {
            Ok(msg) => {
                println!("Response: {:?}", msg);
                Ok(vec![msg])
            }
            Err(err) => {
                println!("Io error: {:?}", err);
                Err(err)
            }
        }
    }
}

#[tokio::main]
pub async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    pretty_env_logger::init();

    let addr = "127.0.0.1:3823";

    let _server = ServerBuilder::from(MyHandlerBuilder { inner: RpcHandler })
        .bind(addr)
        .await?
        .start()
        .await;

    Ok(())
}
