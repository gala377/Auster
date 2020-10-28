use futures::Future;
use tokio::sync::mpsc;
pub mod model;
pub mod runtime;

use crate::room::model::Room;

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct RoomEntry(pub usize);

impl From<&Room> for RoomEntry {
    fn from(room: &Room) -> Self {
        Self(room.id)
    }
}

pub enum RepReq {
    CreateRoom{players_limit: usize, rounds: usize},
    RemoveRoom{room: RoomEntry},
    Close,
}

pub enum RepResp {
    RoomCreated(Room),
    RoomRemoved,
}

pub struct RoomsRepository {
    rooms: Vec<Option<RoomEntry>>,
}

pub type RepReqChannel = mpsc::Sender<(RepReq, mpsc::Sender<RepResp>)>;

impl RoomsRepository {
    pub fn new() -> Self {
        Self { rooms: Vec::new() }
    }

    pub async fn send_req(tx: &mut RepReqChannel, req: RepReq) -> Option<RepResp> {
        let (resp_tx, mut resp_rx) = mpsc::channel(0);
        tx.send((req, resp_tx)).await;
        resp_rx.recv().await
    }

    pub fn new_task() -> (impl Future<Output=()>, RepReqChannel) {
        type ChanData = (RepReq, mpsc::Sender<RepResp>);
        let (tx, mut rx): (mpsc::Sender<ChanData>, mpsc::Receiver<ChanData>) = mpsc::channel(256);
        let mut room_rep = Self::new();
        (async move {
            while let Some((req, mut responder)) = rx.recv().await {
                match req {
                    RepReq::CreateRoom{players_limit, rounds} => {
                        let rd = room_rep.create_room(players_limit, rounds);
                        responder.send(RepResp::RoomCreated(rd)).await;
                    }
                    RepReq::RemoveRoom{room} => {
                        room_rep.remove(room);
                        responder.send(RepResp::RoomRemoved).await;
                    }
                    RepReq::Close => break,
                }
            }
        }, tx)
    }

    pub fn remove(&mut self, room: RoomEntry) {
        let i = match self.find_room_index(room) {
            Some(i) => i,
            None => return,
        };
        self.rooms[i] = None;
    }

    pub fn create_room(&mut self, players_limit: usize, rounds: usize) -> Room {
        match self.find_empty_space() {
            Some(i) => {
                let rd = Room::new(i, players_limit, rounds);
                self.rooms[i] = Some(RoomEntry::from(&rd));
                rd
            }
            None => {
                let rd = Room::new(self.rooms.len(), players_limit, rounds);
                self.rooms.push(Some(RoomEntry::from(&rd)));
                rd
            }
        }
    }

    fn find_empty_space(&self) -> Option<usize> {
        for (i, v) in self.rooms.iter().enumerate() {
            if let None = v {
                return Some(i);
            }
        }
        None
    }

    fn find_room_index(&self, rd: RoomEntry) -> Option<usize> {
        for (i, el) in self.rooms.iter().enumerate() {
            if let Some(el) = el {
                if *el == rd {
                    return Some(i);
                }
            }
        }
        None
    }
}
