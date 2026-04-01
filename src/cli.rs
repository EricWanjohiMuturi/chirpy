use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{http::Http, resp::Resp, rpc::Rpc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Run a RESP Server
    Resp(Resp),
    /// Run a HTTP Server
    Http(Http),
    /// Run an RPC Server
    Rpc(Rpc),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageConfig {
    Sqlite { url: String },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub modes: Vec<Mode>,
    pub storage: StorageConfig,
}

#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to config file
    #[clap(short, long, value_parser, default_value = "chirpy.toml")]
    pub config: PathBuf,
}
