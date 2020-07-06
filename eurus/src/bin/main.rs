use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use serde_json as json;
use tokio::sync::Mutex;
use hyper::{
    Body,
    Request,
    Response,
    Server,
    service::{
        make_service_fn,
        service_fn,
    },
};
use libeurus::{
    room::RoomsRepository,
    service::create_mew_room,
};


#[tokio::main]
async fn main() {
    let room_rep = Arc::new(Mutex::new(RoomsRepository::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let make_svc = make_service_fn(move |_| {
        let rep = Arc::clone(&room_rep);
        async move {
            Ok::<_, Infallible>(service_fn(
                move |req| {
                    let rep = Arc::clone(&rep);
                    async move {
                        handle_req(req, rep).await
                    }
                }
            ))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(shutdown_singal());
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

async fn handle_req(_req: Request<Body>, rep: Arc<Mutex<RoomsRepository>>) -> Result<Response<Body>, Infallible> {
    let body = {
        let mut rep = rep.lock().await;
        create_mew_room(&mut rep)
    };
    let resp = Response::new(
        Body::from(json::to_string(&body).unwrap()));
    Ok(resp)
}


async fn shutdown_singal() {
    tokio::signal::ctrl_c().await
        .expect("failed to install ctrl+c signal handler");
    eprintln!("CTRL+C, shutting down");
}