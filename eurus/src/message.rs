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
    Disconnecting,
}

type PlayerId = usize;

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    RuntimeStarted,
    NewPlayerJoined,    // send public player data to the rest
    PlayerDisconnected, // send disconnected player identifier
    QuestionAdded,
    NewRound,
    GameScore,
    RoomState,
    Err(ErrResponse),
    Priv(PlayerId, Box<Response>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrResponse {
    QuestionLimitReached,
    AnswerAlreadySent,
    AnswerAlreadySelected,
}
