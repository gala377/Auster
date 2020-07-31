use std::{convert::Infallible, net::SocketAddr, path::Path, str::FromStr, sync::Arc};

use fern::colors::{Color, ColoredLevelConfig};
use hyper::{
    http::{response, StatusCode},
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use log::{error, info};
use serde_json as json;
use tokio::sync::Mutex;

use eurus::{config::Config, room::RoomsRepository, service::create_new_room};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage 'eurus config_path'");
        return;
    }
    let path = &args[1];
    let config = match read_config(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("config couldn't be read {}", e);
            return;
        }
    };
    if let Err(e) = setup_logger(&config) {
        eprintln!("logger couldn't be setup {}", e);
        return;
    }
    if let Err(e) = run_server(&config).await {
        error!("server error: {}", e);
    }
}

fn setup_logger(_config: &Config) -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .warn(Color::Yellow)
        .error(Color::Red)
        .info(Color::Green);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}{}[{}][{}] {}\x1B[0m",
                format_args!("\x1B[{}m", colors.get_color(&record.level()).to_fg_str()),
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("hyber", log::LevelFilter::Warn)
        .level_for("paho_mqtt", log::LevelFilter::Warn)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn read_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let bytes = std::fs::read(path)?;
    let contents = std::str::from_utf8(&bytes)?;
    Ok(toml::from_str(contents)?)
}

async fn run_server(config: &Config) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&config.runtime.server_address)?;
    let make_svc = make_service_fn(move |_| {
            let rep = Arc::new(Mutex::new(RoomsRepository::new()));
            let conf = config.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let rep = Arc::clone(&rep);
                    let conf = conf.clone();
                    async move { handle_req(req, rep, conf).await }
                }))
            }
        });
    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(shutdown_singal());
    Ok(server.await?)
}

async fn handle_req(
    _req: Request<Body>,
    rep: Arc<Mutex<RoomsRepository>>,
    config: Config,
) -> Result<Response<Body>, Infallible> {
    let body = {
        // XXX: lock is here just because RoomRepository
        // is a global in memory resource.
        // If we could move to different service or
        // a database for example a lock would be unnecessary.
        let mut rep = rep.lock().await;
        match create_new_room(&mut rep, config) {
            Ok(rd) => rd,
            Err(e) => {
                error!("There was en error while creating a new room: {}", e);
                let resp = response::Builder::new()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("{\"error\": \"Internal server error\"}"))
                    .unwrap();
                return Ok(resp);
            }
        }
    };
    let resp = Response::new(Body::from(json::to_string(&body).unwrap()));
    Ok(resp)
}

async fn shutdown_singal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install ctrl+c signal handler");
    info!("CTRL+C pressed. Shutting down...");
}
