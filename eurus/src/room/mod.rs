use futures::Future;
use tokio::sync::mpsc;
use tracing::warn;
pub mod model;
pub mod runtime;

use crate::{config::Config, db, room::model::Room};

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct RoomEntry(pub usize);

pub struct UserEntry {
    username: u128,
    password: u128,
}

impl From<&Room> for RoomEntry {
    fn from(room: &Room) -> Self {
        Self(room.id)
    }
}

pub enum RepReq {
    CreateRoom { players_limit: usize, rounds: usize },
    RemoveRoom { room: RoomEntry },
    Close,
}

pub enum RepResp {
    RoomCreated(Room),
    RoomRemoved,
    ClosingRepository,
}

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
                        RepReq::RemoveRoom { room } => {
                            room_rep.remove(room);
                            // let us just ignore an error here
                            let _ = responder.send(RepResp::RoomRemoved).await;
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

    fn remove(&mut self, room: RoomEntry) {
        unimplemented!()
    }

    fn create_room(&mut self, players_limit: usize, rounds: usize) -> Room {
        unimplemented!()
    }

    fn create_rt_user(&mut self, room: model::RoomId) -> UserEntry {
        unimplemented!()
    }

    fn create_player_user(&mut self, room: model::RoomId) -> UserEntry {
        unimplemented!()
    }
}
