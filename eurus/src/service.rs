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
    room::{self, RoomData, RoomsRepository},
};

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
) -> Result<RoomData, RoomCreationError> {
    let rd = rep.create_room();
    if let Err(err) = start_room_rt(rd.clone(), config) {
        rep.remove(rd);
        return Err(err);
    }
    Ok(rd)
}

fn start_room_rt(rd: RoomData, config: Config) -> Result<JoinHandle<()>, RoomCreationError> {
    let mut cli = get_mqtt_client(&rd, &config)?;
    let listener = cli.iter_msg();
    let handle = std::thread::spawn(move || {
        info!("[rd-rt-{}] Created new room: {:?}", rd.id, rd);
        debug!("[rd-rt-{}] Waiting for messages", rd.id);
        let runtime = room::runtime::Runtime::new(rd.clone(), config);
        for msg in listener {
            info!("[rd-rt-{}] Got msg", rd.id);
            match runtime.process_msg(&mut cli, msg) {
                ErrorHandling::Abort => {
                    if cli.is_connected() {
                        info!("[rd-rt-{}] Disconnecting", rd.id);
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

fn get_mqtt_client(rd: &RoomData, config: &Config) -> Result<MqttClient, RoomCreationError> {
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
