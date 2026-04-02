mod args;
mod assets;
mod dirs;
mod discord;
mod entity;
mod net;
mod physics;
mod player;
mod renderer;
mod ui;
mod window;
mod world;

use clap::Parser;
use net::connection::ConnectArgs;
use std::path::Path;
use std::sync::Arc;

const SUPPORTED_VERSIONS: &[&str] = &["26.1", "26.1.1-rc-1", "26.1.1"];
const _: () = assert!(!SUPPORTED_VERSIONS.is_empty());

fn rotate_logs(log_dir: &Path) -> std::io::Result<()> {
    let latest = log_dir.join("latest.log");
    if !latest.exists() {
        return Ok(());
    }
    let modified = latest.metadata()?.modified()?;
    let datetime = chrono::DateTime::<chrono::Local>::from(modified);
    let date = datetime.format("%Y-%m-%d");
    let index = (1..)
        .find(|i| !log_dir.join(format!("{date}-{i}.log.gz")).exists())
        .unwrap();
    let dest = log_dir.join(format!("{date}-{index}.log.gz"));
    let input = std::fs::read(&latest)?;
    let output_file = std::fs::File::create(&dest)?;
    let mut encoder = flate2::write::GzEncoder::new(output_file, flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &input)?;
    encoder.finish().map_err(std::io::Error::other)?;
    std::fs::remove_file(&latest)?;
    Ok(())
}

fn main() {
    let args = args::LaunchArgs::parse();

    let version = args
        .version
        .as_deref()
        .unwrap_or_else(|| SUPPORTED_VERSIONS.first().unwrap());

    let data_dirs = dirs::DataDirs::resolve(
        version,
        args.assets_dir.as_deref(),
        args.versions_dir.as_deref(),
        args.game_dir.as_deref(),
    );

    let log_dir = data_dirs.game_dir.join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    if let Err(e) = rotate_logs(&log_dir) {
        eprintln!("Failed to rotate logs: {e}");
    }

    let file_appender = tracing_appender::rolling::never(&log_dir, "latest.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(non_blocking)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    if !cfg!(debug_assertions) && !args.dev {
        match &args.launch_token {
            Some(path) => {
                let token_path = std::path::Path::new(path);
                if !token_path.exists() {
                    eprintln!("Please use the Pomme Launcher to start the game.");
                    std::process::exit(1);
                }
                let _ = std::fs::remove_file(token_path);
            }
            None => {
                eprintln!("Please use the Pomme Launcher to start the game.");
                eprintln!("Download it at: https://github.com/PommeMC/Pomme-Client");
                std::process::exit(1);
            }
        }
    }

    if !SUPPORTED_VERSIONS.contains(&version) {
        tracing::error!(
            "{} is not currently supported. Supported versions: {:?}",
            version,
            SUPPORTED_VERSIONS
        );
        if !cfg!(debug_assertions) && !args.dev {
            std::process::exit(1);
        }
    }

    if let Err(e) = data_dirs.verify() {
        tracing::error!("Failed to verify directories: {e}");
        std::process::exit(1);
    }
    data_dirs.ensure_game_dir().ok();

    tracing::info!("Installation directory: {}", data_dirs.game_dir.display());

    let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"));

    let connection = if let Some(ref server) = args.quick_access_server {
        let connect_args = ConnectArgs {
            server: server.clone(),
            username: args.username.clone().unwrap_or_else(|| "Steve".into()),
            uuid: args
                .uuid
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(uuid::Uuid::nil),
            access_token: args.access_token.clone(),
            view_distance: 12,
        };

        Some(net::connection::spawn_connection(&rt, connect_args))
    } else {
        None
    };

    let launch_auth = match (&args.username, &args.uuid, &args.access_token) {
        (Some(username), Some(uuid_str), Some(token)) => {
            uuid_str.parse().ok().map(|uuid| window::LaunchAuth {
                username: username.clone(),
                uuid,
                access_token: token.clone(),
            })
        }
        _ => None,
    };

    let presence = crate::discord::DiscordPresence::start(version)
        .inspect_err(|e| tracing::warn!("Discord rich presence unavailable: {e}"))
        .ok();

    if let Err(e) = window::run(
        connection,
        version.to_owned(),
        data_dirs,
        rt,
        launch_auth,
        presence,
    ) {
        tracing::error!("Fatal: {e}");
        std::process::exit(1);
    }
}
