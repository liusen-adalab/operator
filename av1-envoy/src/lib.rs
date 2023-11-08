use anyhow::Result;
use utils::logger::{self, Config};
use volo_grpc::Status;

pub mod endpoint;

type RpcResult<T> = Result<volo_grpc::Response<T>, Status>;

pub async fn init_global() -> Result<()> {
    logger::init(&Config {
        level: "debug".to_string(),
    })?;

    Ok(())
}
