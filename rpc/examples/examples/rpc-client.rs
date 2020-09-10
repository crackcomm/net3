//! net3 rpc client example

use net3_msg::compact::Message;
use net3_rpc_examples::{MyMessage, Rpc};
use net3_rpc_server::common::{FromBuilder, NoopHandler};

type Codec = net3_codec_json_lines::Codec<Message>;
type ClientBuilder = net3_rpc_client::ClientBuilder<Codec, FromBuilder<NoopHandler<Message>>>;

#[tokio::main]
pub async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    pretty_env_logger::init();

    let addr = "127.0.0.1:3823";
    let mut client = ClientBuilder::from_addr(addr).background();

    for _ in 0..10 {
        println!(
            "rpc response: {:?}",
            client
                .get_work(&MyMessage {
                    test: "superb".to_owned(),
                })
                .await?
        );
    }
    Ok(())
}
