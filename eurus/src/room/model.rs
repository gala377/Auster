use std::collections::HashMap;

pub type QuestionId = usize;
pub type RoomId = usize;
pub type PlayerId = usize;
pub type AnswerId = usize;

pub struct Room {
    pub id: RoomId, // on room creation
    pub pass: u128, // on room creation
    pub players_limit: usize, // on room creation
    pub players: Vec<Player>,
    pub rounds_limt: usize,
    pub questions: Vec<Question>,
    pub curr_cound: Round,
    pub past_rounds: Vec<Round>,
    pub state: RoomState,
}

pub enum RoomState {
    AcceptingPlayers,
    AcceptingQuestions,
    Playing,
    Dead,
}

pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub points: usize,
}

pub struct Question {
    pub id: QuestionId,
    pub player_id: PlayerId, // who made this question
    pub content: String,
}

pub struct Round {
    pub round_num: usize,
    pub state: RoundState,
    pub question: Question,
    pub answers: HashMap<PlayerId, Answer>,
    pub polls: HashMap<PlayerId, AnswerId>,
}


pub struct Answer {
    pub id: AnswerId,
    pub player_id: PlayerId, // who answered
    pub content: String,
}

pub enum RoundState {
    AcceptingAnswers,
    Polling,
}
