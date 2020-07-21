use std::thread::JoinHandle;

use thiserror::Error;

use crate::message::{
    self,
    mqtt_adapter::{self, MqttError},
    Client, ErrorHandling,
};
use crate::room::{RoomData, RoomsRepository};
use message::ErrorHandler;

#[derive(Error, Debug)]
pub enum RoomCreationError {
    #[error("Connection error: `{0}`")]
    MqqtConnectionError(#[from] MqttError),
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
    let mut cli = mqtt_adapter::MqttClient::new(&rd)?;
    let messages = cli.connect()?;
    cli.subscribe(vec!["test/room/0".into(), "room/0".into()])?;
    let handle = std::thread::spawn(move || {
        println!("[rd-rt-{}] Hey my room is {:?}", rd.id, rd);
        println!("[rd-rt-{}] Waiting for messages", rd.id);
        for msg in messages {
            println!("[rd-rt-{}] Got message", rd.id);
            match msg {
                Err(err) => match mqtt_adapter::MqttErrorHandler::handle_err(&mut cli, err) {
                    ErrorHandling::Abort => {
                        if cli.is_connected() {
                            println!("[rd-rt-{}] Disconnecting", rd.id);
                            // todo: unsunscribe from topics here
                            cli.disconnect().unwrap();
                        }
                        // todo: uncomment if we ever join on errors returned
                        // by handles
                        // return RoomCreationError::from(err);
                        break;
                    }
                    _ => (),
                },
                Ok(msg) => handle_mess(&mut cli, msg),
            }
        }
    });
    Ok(handle)
}

fn handle_mess(_cli: &mut impl message::Client, msg: message::SubMsg) {
    println!("Got msg: {:?}", msg);
}
