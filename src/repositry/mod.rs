use std::path::PathBuf;

use serde::Deserialize;

pub mod host;

#[derive(Debug, Deserialize)]
pub struct SledCfg {
    pub path: PathBuf,
}

trait IdToBytes {
    fn to_bytes(&self) -> Vec<u8>;
}
