use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NewRoomReq {
    pub players_limit: usize,
    pub rounds_limit: usize,
}

#[derive(Debug, Serialize)]
pub struct NewRoomResp {
    pub id: String,
    pub password: i64,
}


#[derive(Debug, Serialize)]
pub struct NewPlayerReq {
    id: usize,
    password: u64,
    name: String,
}
