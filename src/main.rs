mod args;
mod assets;
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

fn default_minecraft_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".minecraft")
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("Library/Application Support/minecraft")
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".minecraft")
    }
}

fn main() {
    env_logger::init();

    let args = args::LaunchArgs::parse();

    let assets_dir: PathBuf = args
        .assets_dir
        .as_deref()
        .unwrap_or("reference/assets")
        .into();

    let game_dir: PathBuf = args
        .game_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(default_minecraft_dir);

    let rt = Arc::new(tokio::runtime::Runtime::new().expect("failed to create tokio runtime"));

    let connection = if let Some(ref server) = args.server {
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

    if let Err(e) = window::run(connection, assets_dir, game_dir, rt) {
        log::error!("Fatal: {e}");
        std::process::exit(1);
    }
}
