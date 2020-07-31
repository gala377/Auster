use log::{error, info};

use crate::{
    client::{
        self,
        mqtt::{MqttClient, MqttError, MqttErrorHandler},
        ErrorHandler, ErrorHandling,
    },
    config::Config,
    message,
    room::RoomData,
};

pub struct Runtime {
    _rd: RoomData,
    _config: Config,
}

type RawMsg = Result<(String, message::SubMsg), MqttError>;

impl Runtime {
    pub fn new(rd: RoomData, config: Config) -> Self {
        Self {
            _rd: rd,
            _config: config,
        }
    }

    pub fn process_msg(&self, cli: &mut MqttClient, msg: RawMsg) -> ErrorHandling {
        match msg {
            Err(err) => MqttErrorHandler::handle_err(cli, err),
            Ok((channel, msg)) => handle_mess(cli, msg, channel),
        }
    }
}

fn handle_mess(
    cli: &mut impl client::Client,
    msg: message::SubMsg,
    channel: String,
) -> ErrorHandling {
    info!("Got msg: {:?} from channel {}", msg, channel);
    if let Err(e) = cli.publish(channel, message::PubMsg::Hey.into()) {
        error!("couldn't publish message {}", e);
    }
    ErrorHandling::Skip
}
