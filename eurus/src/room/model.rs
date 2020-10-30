use crate::repository::EntryId;
use std::collections::HashMap;

pub type QuestionId = usize;
pub type RoomId = EntryId;
pub type PlayerId = usize;
pub type PlayerToken = usize;
pub type AnswerId = usize;

pub struct Room {
    pub id: RoomId,           // on room creation
    pub pass: i64,            // on room creation
    pub players_limit: usize, // on room creation
    pub players: Vec<Player>,
    pub rounds_limit: usize,
    pub questions: Vec<Question>,
    pub curr_round: Option<Round>,
    pub past_rounds: Vec<Round>,
    pub state: RoomState,
}

impl Room {
    pub fn new(id: RoomId, password: i64, players_limit: usize, rounds_limit: usize) -> Self {
        Self {
            id,
            players_limit,
            rounds_limit,
            pass: password,
            players: Vec::new(),
            questions: Vec::new(),
            past_rounds: Vec::new(),
            curr_round: None,
            state: RoomState::AcceptingPlayers,
        }
    }
}

pub enum RoomState {
    AcceptingPlayers,
    AcceptingQuestions,
    Playing,
    Dead,
}

pub struct Player {
    pub id: PlayerId,
    pub token: PlayerToken,
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
