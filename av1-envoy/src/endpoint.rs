use tracing::debug;
use volo_gen::av1::operator::{self, Ping, Pong};
use volo_grpc::{Request, Response};

use crate::RpcResult;

pub struct Host;

impl operator::NodeService for Host {
    async fn ping(&self, req: Request<Ping>) -> RpcResult<Pong> {
        debug!(?req, "ping");
        Ok(Response::new(Pong {
            message: req.into_inner().message,
        }))
    }
}
