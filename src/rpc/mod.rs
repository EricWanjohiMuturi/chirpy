use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct Rpc {
    host: String,
    port: String
}
