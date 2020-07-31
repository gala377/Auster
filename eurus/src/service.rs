use std::thread::JoinHandle;

use log::{debug, error, info};
use thiserror::Error;

use crate::{
    client::{
        mqtt::{MqttClient, MqttError},
        Client, ErrorHandling,
    },
    config::Config,
    message,
    room::{self, RoomsRepository},
};
use room::{model::Room, RoomEntry};

pub mod dto;

#[derive(Error, Debug)]
pub enum RoomCreationError {
    #[error("{0}")]
    MqttConnectionError(#[from] MqttError),
    #[error("Could not encode the message {0}")]
    MsgEncodingError(#[from] serde_json::Error),
}

pub fn create_new_room(
    rep: &mut RoomsRepository,
    config: Config,
    room_req: dto::NewRoomReq,
) -> Result<dto::NewRoomResp, RoomCreationError> {
    let rd = rep.create_room(room_req.players_limit, room_req.rounds_limit);
    let re = RoomEntry::from(&rd);
    let resp = dto::NewRoomResp::from(&rd);
    if let Err(err) = start_room_rt(rd, config) {
        rep.remove(re);
        return Err(err);
    }
    Ok(resp)
}

fn start_room_rt(rd: Room, config: Config) -> Result<JoinHandle<()>, RoomCreationError> {
    let mut cli = get_mqtt_client(&rd, &config)?;
    let listener = cli.iter_msg();
    let handle = std::thread::spawn(move || {
        info!("[rd-rt-{}] Created new room", rd.id);
        debug!("[rd-rt-{}] Waiting for messages", rd.id);
        let rd_id = rd.id;
        let runtime = room::runtime::Runtime::new(rd, config);
        for msg in listener {
            info!("[rd-rt-{}] Got msg", rd_id);
            match runtime.process_msg(&mut cli, msg) {
                ErrorHandling::Abort => {
                    if cli.is_connected() {
                        info!("[rd-rt-{}] Disconnecting", rd_id);
                        // todo: unsubscribe from topics here
                        cli.disconnect().unwrap();
                    }
                    break;
                }
                _ => (),
            }
        }
    });
    Ok(handle)
}

fn get_mqtt_client(rd: &Room, config: &Config) -> Result<MqttClient, RoomCreationError> {
    let channel_prefix = &config.runtime.room_channel_prefix;
    let mut cli = MqttClient::new(rd, &config.mqtt.host)?;
    cli.connect()?;
    cli.subscribe(vec![format!("{}/{}/rt/write", channel_prefix, rd.id)])?;
    cli.publish(
        format!("{}/{}/rt/read", channel_prefix, rd.id),
        serde_json::to_string(&message::Response::RuntimeStarted)?,
    )?;
    Ok(cli)
}
