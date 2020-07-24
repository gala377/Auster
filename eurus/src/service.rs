use std::thread::JoinHandle;

use log::{debug, error, info};
use thiserror::Error;

use crate::message::{
    self,
    mqtt_adapter::{self, MqttError},
    Client, ErrorHandler, ErrorHandling,
};
use crate::{
    config::Config,
    room::{RoomData, RoomsRepository},
};

#[derive(Error, Debug)]
pub enum RoomCreationError {
    #[error("{0}")]
    MqqtConnectionError(#[from] MqttError),
}

pub fn create_new_room(
    rep: &mut RoomsRepository,
    config: Config,
) -> Result<RoomData, RoomCreationError> {
    let rd = rep.create_room();
    use RoomCreationError::*;
    if let Err(err @ MqqtConnectionError(_)) = start_room_rt(rd.clone(), config) {
        rep.remove(rd);
        return Err(err);
    }
    Ok(rd)
}

macro_rules! exec_err_strategy {
    ($rd:expr, $cli:expr, $e:expr) => {
        match $e {
            ErrorHandling::Abort => {
                if $cli.is_connected() {
                    info!("[rd-rt-{}] Disconnecting", $rd.id);
                    // todo: unsubscribe from topics here
                    $cli.disconnect().unwrap();
                }
                // todo: uncomment if we ever join on errors returned
                // by handles
                // return RoomCreationError::from(err);
                break;
            }
            _ => (),
        }
    };
}

fn start_room_rt(rd: RoomData, config: Config) -> Result<JoinHandle<()>, RoomCreationError> {
    let mut cli = mqtt_adapter::MqttClient::new(&rd, config.mqtt.host)?;
    let messages = cli.connect()?;
    cli.subscribe(vec![format!(
        "{}/{}",
        config.runtime.room_channel_prefix, rd.id
    )
    .into()])?;
    cli.publish(
        format!("{}/{}", config.runtime.room_channel_prefix, rd.id),
        message::PubMsg::Hey.into(),
    )?;

    let handle = std::thread::spawn(move || {
        info!("[rd-rt-{}] Created new room: {:?}", rd.id, rd);
        debug!("[rd-rt-{}] Waiting for messages", rd.id);
        for msg in messages {
            info!("[rd-rt-{}] Got message", rd.id);
            match msg {
                Err(err) => exec_err_strategy!(
                    rd,
                    cli,
                    mqtt_adapter::MqttErrorHandler::handle_err(&mut cli, err)
                ),
                Ok((channel, msg)) => {
                    exec_err_strategy!(rd, cli, handle_mess(&mut cli, msg, channel))
                }
            }
        }
    });
    Ok(handle)
}

fn handle_mess(
    cli: &mut impl message::Client,
    msg: message::SubMsg,
    channel: String,
) -> ErrorHandling {
    info!("Got msg: {:?} from channel {}", msg, channel);
    if let Err(e) = cli.publish(channel, message::PubMsg::Hey.into()) {
        error!("couldn't publish message {}", e);
    }
    ErrorHandling::Skip
}
