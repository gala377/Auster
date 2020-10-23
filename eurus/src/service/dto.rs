use crate::room::model::Room;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NewRoomReq {
    pub players_limit: usize,
    pub rounds_limit: usize,
}

#[derive(Debug, Serialize)]
pub struct NewRoomResp {
    id: usize,
    password: u64,
    players_limit: usize,
    round_limit: usize,
}

impl From<&Room> for NewRoomResp {
    fn from(room: &Room) -> Self {
        Self {
            id: room.id,
            password: room.pass,
            players_limit: room.players_limit,
            round_limit: room.rounds_limit,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NewPlayerReq {
    id: usize,
    password: u64,
    name: String,
}

