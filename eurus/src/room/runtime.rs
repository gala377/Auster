use log::info;

use super::model::Room;
use crate::{
    client::{
        self,
        mqtt::{MqttClient, MqttError, MqttErrorHandler},
        ErrorHandler, ErrorHandling,
    },
    config::Config,
    message,
};

pub struct Runtime {
    _rd: Room,
    _config: Config,
}

type RawMsg = Result<(String, message::Request), MqttError>;

impl Runtime {
    pub fn new(rd: Room, config: Config) -> Self {
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
    _cli: &mut impl client::Client,
    msg: message::Request,
    channel: String,
) -> ErrorHandling {
    info!("Got msg: {:?} from channel {}", msg, channel);
    ErrorHandling::Skip
}
