use std::net::SocketAddr;

use tracing::info;
use volo_gen::av1::operator::NodeServiceServer;
use volo_grpc::server::{Server, ServiceBuilder};

use av1_envoy::endpoint::Host;

#[volo::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = "0.0.0.0:18989".parse().unwrap();
    let bind = volo::net::Address::from(addr);
    av1_envoy::init_global().await?;

    info!(?bind, "start server");
    Server::new()
        .add_service(ServiceBuilder::new(NodeServiceServer::new(Host)).build())
        .run(bind)
        .await
        .unwrap();

    Ok(())
}
