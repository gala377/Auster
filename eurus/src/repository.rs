use futures::Future;
use mongodb::bson::doc;
use tokio::sync::mpsc;
use tracing::warn;

use crate::{config::Config, db, room::model};

pub type EntryId = [u8; 12];
pub struct UserEntry {
    pub username: EntryId,
    pub password: i64, // todo: bytes maybe?
}

pub struct RoomEntry {
    pub id: EntryId,
    pub password: i64, // todo: change it to bytes maybe?
}

pub enum RepReq {
    CreateRoom { players_limit: usize },
    RemoveRoom { room_id: EntryId },
    CreateRuntimeUser { room_id: EntryId },
    CreatePlayerUser { room_id: EntryId },
    Close,
}

pub enum RepResp {
    RoomCreated(RoomEntry),
    RoomRemoved,
    ClosingRepository,
    UserCreated(UserEntry),
}

pub enum RepError {
    ChannelClosed,
}

pub type RepReqChannel = mpsc::Sender<(RepReq, mpsc::Sender<Result<RepResp, RepError>>)>;
pub struct DataRepository {
    conn: db::Connection,
}

impl DataRepository {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        Ok(Self {
            conn: db::Connection::new(config).await?,
        })
    }

    pub async fn send_req(tx: &mut RepReqChannel, req: RepReq) -> Result<RepResp, RepError> {
        let (resp_tx, mut resp_rx) = mpsc::channel(1);
        if let Err(err) = tx.send((req, resp_tx)).await {
            warn!("could not send a command to room repository {}", err);
            return Err(RepError::ChannelClosed);
        }
        match resp_rx.recv().await {
            Some(val) => val,
            None => Err(RepError::ChannelClosed),
        }
    }

    pub async fn new_task(
        config: &Config,
    ) -> anyhow::Result<(impl Future<Output = ()>, RepReqChannel)> {
        type ChanData = (RepReq, mpsc::Sender<Result<RepResp, RepError>>);
        let (tx, mut rx): (mpsc::Sender<ChanData>, mpsc::Receiver<ChanData>) = mpsc::channel(256);
        let mut room_rep = Self::new(config).await?;
        Ok((
            async move {
                while let Some((req, mut responder)) = rx.recv().await {
                    match req {
                        RepReq::CreateRoom { players_limit } => {
                            let rd = room_rep.create_room(players_limit).await;
                            // let us just ignore an error here
                            let _ = responder.send(Ok(RepResp::RoomCreated(rd))).await;
                        }
                        RepReq::RemoveRoom { room_id } => {
                            room_rep.remove_room(room_id).await;
                            // let us just ignore an error here
                            let _ = responder.send(Ok(RepResp::RoomRemoved)).await;
                        }
                        RepReq::CreateRuntimeUser { room_id } => {
                            let ud = room_rep.create_rt_user(room_id).await;
                            let _ = responder.send(Ok(RepResp::UserCreated(ud))).await;
                        }
                        RepReq::CreatePlayerUser { room_id } => {
                            let ud = room_rep.create_player_user(room_id).await;
                            let _ = responder.send(Ok(RepResp::UserCreated(ud))).await;
                        }
                        RepReq::Close => {
                            // Note that it does some cleanup after sending the message and whats
                            // more it even yields here so repositories task should still
                            // be awaited instead of using a channel.
                            rx.close();
                            let _ = responder.send(Ok(RepResp::ClosingRepository)).await;
                            // We do not break from while as we still want to drain messages
                        }
                    }
                }
            },
            tx,
        ))
    }

    async fn create_room(&mut self, players_limit: usize) -> RoomEntry {
        let room_pass: i64 = rand::random();
        let insert_res = self
            .conn
            .rooms_col
            .insert_one(
                doc! {
                    "room_pass": room_pass,
                    "players_limit": players_limit as i64,
                    "curr_players": 0_i32,
                },
                None,
            )
            .await;
        let insert_res = match insert_res {
            Ok(val) => val,
            Err(err) => {
                // todo: error handling
                panic!("That should not have happened {}", err);
            }
        };
        let id = insert_res
            .inserted_id
            .as_object_id()
            .expect("bson object returned by insert_one should be an ObjectId")
            .bytes();
        RoomEntry {
            id,
            password: room_pass,
        }
    }

    async fn remove_room(&mut self, room: model::RoomId) {
        // todo: implement
        warn!("Removing room {}", base64::encode(&room));
    }

    async fn create_rt_user(&mut self, _room: model::RoomId) -> UserEntry {
        unimplemented!()
    }

    async fn create_player_user(&mut self, _room: model::RoomId) -> UserEntry {
        unimplemented!()
    }
}
