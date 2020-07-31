use serde::{Deserialize, Serialize};

//
// WIP: Messages
//
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    GetRoomState, // so the client can get the latest state if they wish to
    JoinRoom,     // send player data associated with this
    AddQuestion,
    AddAnswer,
    SelectAnswer,
    Disconneting,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    RuntimeStarted,
    NewPlayerJoined,    // send public player data to the rest
    PlayerDisconnected, // send disconnected player identifier
    QuestionAdded,
    NewRound,
    GameScore,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PrivResponse {
    Err(ErrResponse),
    RoomState,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrResponse {
    QuestionLimitReached,
    AnswerAlreadySent,
    AnswerAlreadySelected,
}
