use std::convert::Infallible;
use std::net::SocketAddr;
use std::{sync::Arc};

use serde_json as json;
use tokio::sync::Mutex;
use hyper::{
    Body,
    Request,
    Response,
    http::{
        response,
        StatusCode,
    },
    Server,
    service::{
        make_service_fn,
        service_fn,
    },
};
use libeurus::{
    room::RoomsRepository,
    service::create_new_room,
};


#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let make_svc = {
        let room_rep = Arc::new(Mutex::new(RoomsRepository::new()));
        make_service_fn(move |_| {
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
        })
    };
    let server = Server::bind(&addr).serve(make_svc);
    let server = server.with_graceful_shutdown(shutdown_singal());
    println!("Starting server");
    if let Err(e) = server.await {
        println!("server error: {}", e);
    }
}

async fn handle_req(_req: Request<Body>, rep: Arc<Mutex<RoomsRepository>>) -> Result<Response<Body>, Infallible> {
    let body = {
        let mut rep = rep.lock().await;
        match create_new_room(&mut rep) {
            Ok(rd) => rd,
            Err(_e) => {
                // todo: Write error msg
                println!("There was en error while creating a new room");
                let resp = response::Builder::new()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("{\"error\": \"Internal server error\"}"))
                    .unwrap();
                return Ok(resp);
            }
        }
    };
    let resp = Response::new(
        Body::from(json::to_string(&body).unwrap()));
    Ok(resp)
}


async fn shutdown_singal() {
    tokio::signal::ctrl_c().await
        .expect("failed to install ctrl+c signal handler");
    println!("CTRL+C, shutting down");
}