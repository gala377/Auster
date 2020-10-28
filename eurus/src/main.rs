use std::{convert::Infallible, net::SocketAddr, path::Path, str::FromStr};
use tracing::Instrument;

use futures::TryStreamExt;

use hyper::{
    http::{response, StatusCode},
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use tracing::{error, info};

use eurus::{
    config::Config,
    room::{RepReq, RepReqChannel, RoomsRepository},
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
    let (task, mut room_rep_chan) = RoomsRepository::new_task();
    let room_rep_task = tokio::spawn(task);
    if let Err(e) = run_server(&config, room_rep_chan.clone()).await {
        eprintln!("server error: {}", e);
    }
    RoomsRepository::send_req(&mut room_rep_chan, RepReq::Close).await;
    if let Err(err) = room_rep_task.await {
        eprintln!("couldn't join on the repository task: {}", err);
    }
}

fn setup_logger(_config: &Config) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_timer(tracing_subscriber::fmt::time())
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .compact()
        .try_init()
        .unwrap(); // todo: yeah, not the best solution, for now it's ok
    Ok(())
}

fn read_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let bytes = std::fs::read(path)?;
    let contents = std::str::from_utf8(&bytes)?;
    Ok(toml::from_str(contents)?)
}

#[tracing::instrument(skip(rep))]
async fn run_server(config: &Config, rep: RepReqChannel) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&config.runtime.server_address)?;
    let make_svc = make_service_fn(move |_| {
        // Why do we need 2 levels of clone?
        let conf = config.clone(); // <---- one here
        let span = tracing::debug_span!("service creation");
        let rep_clone = rep.clone(); // <----- here
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let span = tracing::debug_span!("request span");
                let rep = rep_clone.clone(); // <--- and the second one here
                let conf = conf.clone(); // <---- here
                async move { Ok::<_, Infallible>(handle_req(req, rep, conf).await) }
                    .instrument(span)
            }))
        }
        .instrument(span)
    });
    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(shutdown_singal());
    Ok(server.await?)
}

#[tracing::instrument(skip(rep))]
async fn handle_req(req: Request<Body>, rep: RepReqChannel, config: Config) -> Response<Body> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/new_room") => new_room(req, rep, config).await,
        _ => error_response("not found", StatusCode::NOT_FOUND),
    }
}

#[tracing::instrument(skip(rep))]
async fn new_room(req: Request<Body>, rep: RepReqChannel, config: Config) -> Response<Body> {
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
    match create_new_room(rep, config, body).await {
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

#[tracing::instrument(skip(msg))]
fn error_response(msg: impl AsRef<str>, status: StatusCode) -> Response<Body> {
    response::Builder::new()
        .status(status)
        .body(Body::from(format!("{{\"error\": \"{}\"}}", msg.as_ref())))
        .unwrap()
}

#[tracing::instrument]
async fn shutdown_singal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install ctrl+c signal handler");
    info!("CTRL+C pressed. Shutting down...");
}
