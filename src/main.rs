mod args;
mod net;
mod physics;
mod player;
mod renderer;
mod ui;
mod window;
mod world;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;

use net::connection::ConnectArgs;

fn main() {
    env_logger::init();

    let args = args::LaunchArgs::parse();

    let assets_dir: PathBuf = args
        .assets_dir
        .as_deref()
        .unwrap_or("reference/assets")
        .into();

    let rt = Arc::new(tokio::runtime::Runtime::new().expect("failed to create tokio runtime"));

    let event_rx = if let Some(ref server) = args.server {
        let connect_args = ConnectArgs {
            server: server.clone(),
            username: args.username.clone().unwrap_or_else(|| "Steve".into()),
            uuid: args
                .uuid
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(uuid::Uuid::nil),
            access_token: args.access_token.clone(),
        };

        Some(net::connection::spawn_connection(&rt, connect_args))
    } else {
        None
    };

    if let Err(e) = window::run(event_rx, assets_dir, rt) {
        log::error!("Fatal: {e}");
        std::process::exit(1);
    }
}
