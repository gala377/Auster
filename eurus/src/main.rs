use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use fern::colors::{Color, ColoredLevelConfig};
use hyper::{
    http::{response, StatusCode},
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use log::{error, info};
use serde_json as json;
use tokio::sync::Mutex;

use eurus::{room::RoomsRepository, service::create_new_room};

#[tokio::main]
async fn main() {
    if let Err(e) = setup_logger() {
        eprintln!("logger couldn't be setup {}", e);
        return;
    }
    if let Err(e) = run_server().await {
        error!("server error: {}", e);
    }
}

fn setup_logger() -> Result<(), fern::InitError> {
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

async fn run_server() -> Result<(), hyper::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let make_svc = {
        let room_rep = Arc::new(Mutex::new(RoomsRepository::new()));
        make_service_fn(move |_| {
            let rep = Arc::clone(&room_rep);
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let rep = Arc::clone(&rep);
                    async move { handle_req(req, rep).await }
                }))
            }
        })
    };
    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(shutdown_singal());
    server.await
}

async fn handle_req(
    _req: Request<Body>,
    rep: Arc<Mutex<RoomsRepository>>,
) -> Result<Response<Body>, Infallible> {
    let body = {
        // XXX: lock is here just because RoomRepository
        // is a global in memory resource.
        // If we could move to different service or
        // a database for example a lock would not we needed.
        let mut rep = rep.lock().await;
        match create_new_room(&mut rep) {
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
