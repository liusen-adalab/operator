use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};
use tracing::debug;
use utils::id_new_type;
use volo_gen::av1::operator::{NodeServiceClient, NodeServiceClientBuilder, Ping};

use crate::settings::get_settings;

pub mod convert;
pub mod http_enpoint;
pub mod ssh;

id_new_type!(HostId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: HostId,
    pub name: String,
    pub ip: IpAddr,
    pub state: HostState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostState {
    Running,
    Stopped,
    Disconnected,
}

impl Host {
    pub async fn ping(&mut self) {
        let client = self.client();
        let pong = client.ping(Ping { message: "ping".into() }).await;
        if pong.is_err() {
            debug!(?pong, "ping host error");
            self.state = HostState::Disconnected;
        } else {
            self.state = HostState::Running;
        }

        debug!(?pong);
    }

    fn client(&self) -> NodeServiceClient {
        let port = get_settings().envoy.port;
        let addr = SocketAddr::new(self.ip, port);
        debug!(?addr, "create grpc client");
        let client = NodeServiceClientBuilder::new("av1-operator").address(addr).build();
        client
    }
}
