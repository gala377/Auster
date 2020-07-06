use paho_mqtt as mqtt;

use crate::room::{RoomData, RoomsRepository};


pub fn create_mew_room(rep: &mut RoomsRepository) -> RoomData {
    let rd = rep.create_room();
    spawn_new_room_thread(rd.clone());
    rd
}

fn spawn_new_room_thread(rd: RoomData) {
    let cli = mqtt::Client::new("tcp://localhost:1883")
        .expect("Could not connect to mqtt broker");
    cli.set_timeout(Duration::from_secs(5))
    let conn_options = mqtt::ConnectOptions::new();
    cli.connect(conn_options).
    let _handle = std::thread::spawn(move || {
    });
}