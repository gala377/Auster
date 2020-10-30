use crate::repository::RoomEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NewRoomReq {
    pub players_limit: usize,
    pub rounds_limit: usize,
}

#[derive(Debug, Serialize)]
pub struct NewRoomResp {
    id: [u8; 12],
    password: i64,
}

impl From<&RoomEntry> for NewRoomResp {
    fn from(room: &RoomEntry) -> Self {
        Self {
            id: room.id,
            password: room.password,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NewPlayerReq {
    id: usize,
    password: u64,
    name: String,
}
