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

pub struct RoomsRepository {
    rooms: Vec<Option<RoomEntry>>,
}

impl RoomsRepository {
    pub fn new() -> Self {
        Self { rooms: Vec::new() }
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
