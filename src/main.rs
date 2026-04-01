use apalis::prelude::*;
use clap::Parser;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};

use crate::{
    cli::{Cli, Config},
    server::run_server,
};

mod cli;
mod http;
mod job;
mod resp;
mod rpc;
mod server;
mod storage;

#[tokio::main]
async fn main() -> Result<(), BoxDynError> {
    let cli = Cli::parse();
    let config: Config = Figment::new()
        .merge(Env::prefixed("CHIRPY_"))
        .merge(Toml::file(cli.config))
        .extract()?;

    run_server(config).await
}
