use rand;

use serde::{Serialize, Deserialize};

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoomData {
    pub id: usize,
    pub pass: u64,
}

impl RoomData {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            pass: Self::gen_random_pass(),
        }
    }

    fn gen_random_pass() -> u64 {
        rand::random()
    }
}

pub struct RoomsRepository {
    rooms: Vec<Option<RoomData>>,
}


impl RoomsRepository {
    pub fn new() -> Self {
        Self { rooms: Vec::new() }
    }

    pub fn remove(&mut self, room: RoomData) {
        let i = match self.find_room_index(room) {
            Some(i) => i,
            None => return,
        };
        self.rooms[i] = None;
    }

    pub fn create_room(&mut self) -> RoomData {
        match self.find_empy_space() {
            Some(i) => {
                let rd = RoomData::new(i);
                self.rooms[i] = Some(rd.clone());
                rd
            },
            None => {
                let rd = RoomData::new(self.rooms.len());
                self.rooms.push(Some(rd.clone()));
                rd
            }
        } 
    }

    fn find_empy_space(&self) -> Option<usize> {
        for (i, v) in self.rooms.iter().enumerate() {
            if let None = v {
                return Some(i);
            }
        }
        None
    }

    fn find_room_index(&self, rd: RoomData) -> Option<usize> {
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



