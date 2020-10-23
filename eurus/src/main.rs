use tokio::sync::Mutex;
use std::{convert::Infallible, net::SocketAddr, path::Path, str::FromStr, sync::Arc};

use futures::TryStreamExt;

use fern::colors::{Color, ColoredLevelConfig};
use hyper::{
    http::{response, StatusCode},
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use log::{error, info};

use eurus::{
    config::Config,
    room::RoomsRepository,
    service::{create_new_room, dto},
};

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
        // todo: Does this work? Doesn't this create new repository each time
        // we get a new request?
        let rep = Arc::new(Mutex::new(RoomsRepository::new()));
        let conf = config.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let rep = Arc::clone(&rep);
                let conf = conf.clone();
                async move { Ok::<_, Infallible>(handle_req(req, rep, conf).await) }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(shutdown_singal());
    Ok(server.await?)
}

async fn handle_req(
    req: Request<Body>,
    rep: Arc<Mutex<RoomsRepository>>,
    config: Config,
) -> Response<Body> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/new_room") => new_room(req, rep, config).await,
        _ => error_response("not found", StatusCode::NOT_FOUND),
    }
}

async fn new_room(
    req: Request<Body>,
    rep: Arc<Mutex<RoomsRepository>>,
    config: Config,
) -> Response<Body> {
    // todo: check if both are within limits
    let (_, body) = req.into_parts();
    let body = match body
        .try_fold(Vec::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
    {
        Ok(val) => val,
        Err(_) => return error_response("could not assemble message", StatusCode::BAD_REQUEST),
    };
    let body: dto::NewRoomReq = match serde_json::from_slice(&body) {
        Ok(val) => val,
        Err(_) => return error_response("could not decode message", StatusCode::BAD_REQUEST),
    };
    // todo: Change this.
    // XXX: lock is here just because RoomRepository
    // is a global in memory resource.
    // If we could move to different service or
    // a database for example a lock would be unnecessary.
    // We could spawn a task to manage database writes
    // with message passing. That is what tokio docs want you to do
    // or create a struct holding a synchronous mutex and performing
    // only synchronous operations on it so that lock is not held
    // across multiple await points.
    // lock for the whole room creation process is really excessive.
    let mut rep = rep.lock().await;
    match create_new_room(&mut *rep, config, body).await {
        Ok(rd) => {
            let body = match serde_json::to_vec(&rd) {
                Ok(val) => val,
                Err(_) => {
                    return error_response(
                        "internal server error",
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            };
            Response::new(Body::from(body))
        }
        Err(e) => {
            error!("There was en error while creating a new room: {}", e);
            error_response("internal server error", StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn error_response(msg: impl AsRef<str>, status: StatusCode) -> Response<Body> {
    response::Builder::new()
        .status(status)
        .body(Body::from(format!("{{\"error\": \"{}\"}}", msg.as_ref())))
        .unwrap()
}

async fn shutdown_singal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install ctrl+c signal handler");
    info!("CTRL+C pressed. Shutting down...");
}
