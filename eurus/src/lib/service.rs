use std::error::Error;
use std::thread::JoinHandle;

use crate::message::{self, mqtt_adapter, Client};
use crate::room::{RoomData, RoomsRepository};

pub enum RoomCreationError {
    MqqtConnectionError(Box<dyn Error>),
}

pub fn create_new_room(rep: &mut RoomsRepository) -> Result<RoomData, RoomCreationError> {
    let rd = rep.create_room();
    use RoomCreationError::*;
    if let Err(err @ MqqtConnectionError(_)) = start_room_rt(rd.clone()) {
        rep.remove(rd);
        return Err(err);
    }
    Ok(rd)
}

fn start_room_rt(rd: RoomData) -> Result<JoinHandle<()>, RoomCreationError> {
    let mut cli = match mqtt_adapter::MqttClient::new(&rd) {
        Ok(cli) => cli,
        Err(err) => return Err(RoomCreationError::MqqtConnectionError(err)),
    };
    let messages = match cli.connect() {
        Ok(rx) => rx,
        Err(err) => return Err(RoomCreationError::MqqtConnectionError(err)),
    };
    if let Err(err) = cli.subscribe(vec!["test/room/0".into(), "room/0".into()]) {
        return Err(RoomCreationError::MqqtConnectionError(err));
    }
    let handle = std::thread::spawn(move || {
        println!("[rd-rt-{}] Hey my room is {:?}", rd.id, rd);
        println!("[rd-rt-{}] Waiting for messages", rd.id);
        for msg in messages {
            println!("[rd-rt-{}] Got message", rd.id);
            if let Some(msg) = msg {
                handle_mess(&mut cli, msg);
            } else if cli.is_connected() || !cli.try_reconnect() {
                println!("[rd-rt-{}] channel died", rd.id);
                // channel died
                break;
            }
        }
        if cli.is_connected() {
            println!("[rd-rt-{}] Disconnecting", rd.id);
            // todo: unsunscribe from topics here
            cli.disconnect().unwrap();
        }
    });
    Ok(handle)
}

fn handle_mess(_cli: &mut impl message::Client, msg: message::SubMsg) {
    println!("Got msg: {:?}", msg);
}
