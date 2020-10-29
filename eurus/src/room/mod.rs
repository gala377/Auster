use futures::Future;
use tokio::sync::mpsc;
use tracing::warn;
pub mod model;
pub mod runtime;

use crate::{config::Config, db, room::model::Room};

pub struct UserEntry {
    username: u128,
    password: u128,
}

pub enum RepReq {
    CreateRoom { players_limit: usize, rounds: usize },
    RemoveRoom { room_id: model::RoomId },
    CreateRuntimeUser { room_id: model::RoomId },
    CreatePlayerUser { room_id: model::RoomId },
    Close,
}

pub enum RepResp {
    RoomCreated(Room),
    RoomRemoved,
    ClosingRepository,
    UserCreated(UserEntry),
    Err(RepError),
}

pub struct RepError;

pub type RepReqChannel = mpsc::Sender<(RepReq, mpsc::Sender<RepResp>)>;
pub struct DataRepository {
    conn: db::Connection,
}

impl DataRepository {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        Ok(Self {
            conn: db::Connection::new(config).await?,
        })
    }

    pub async fn send_req(tx: &mut RepReqChannel, req: RepReq) -> Option<RepResp> {
        let (resp_tx, mut resp_rx) = mpsc::channel(1);
        if let Err(err) = tx.send((req, resp_tx)).await {
            warn!("could not send a command to room repository {}", err);
            return None;
        }
        resp_rx.recv().await
    }

    pub async fn new_task(
        config: &Config,
    ) -> anyhow::Result<(impl Future<Output = ()>, RepReqChannel)> {
        type ChanData = (RepReq, mpsc::Sender<RepResp>);
        let (tx, mut rx): (mpsc::Sender<ChanData>, mpsc::Receiver<ChanData>) = mpsc::channel(256);
        let mut room_rep = Self::new(config).await?;
        Ok((
            async move {
                while let Some((req, mut responder)) = rx.recv().await {
                    match req {
                        RepReq::CreateRoom {
                            players_limit,
                            rounds,
                        } => {
                            let rd = room_rep.create_room(players_limit, rounds);
                            // let us just ignore an error here
                            let _ = responder.send(RepResp::RoomCreated(rd)).await;
                        }
                        RepReq::RemoveRoom { room_id } => {
                            room_rep.remove_room(room_id);
                            // let us just ignore an error here
                            let _ = responder.send(RepResp::RoomRemoved).await;
                        }
                        RepReq::CreateRuntimeUser { room_id } => {
                            let ud = room_rep.create_rt_user(room_id);
                            let _ = responder.send(RepResp::UserCreated(ud)).await;
                        }
                        RepReq::CreatePlayerUser { room_id } => {
                            let ud = room_rep.create_player_user(room_id);
                            let _ = responder.send(RepResp::UserCreated(ud)).await;
                        }
                        RepReq::Close => {
                            // Note that it does some cleanup after sending the message and whats
                            // more it even yields here so repositories task should still
                            // be awaited instead of using a channel.
                            rx.close();
                            let _ = responder.send(RepResp::ClosingRepository).await;
                            // We do not break from while as we still want to drain messages
                        }
                    }
                }
            },
            tx,
        ))
    }

    fn create_room(&mut self, players_limit: usize, rounds: usize) -> Room {
        unimplemented!()
    }

    fn remove_room(&mut self, room: model::RoomId) {
        unimplemented!()
    }

    fn create_rt_user(&mut self, room: model::RoomId) -> UserEntry {
        unimplemented!()
    }

    fn create_player_user(&mut self, room: model::RoomId) -> UserEntry {
        unimplemented!()
    }
}
