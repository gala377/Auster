use log::info;

use super::model::Room;
use crate::{
    config::Config,
    message,
};

pub struct Runtime {
    _rd: Room,
    _config: Config,
}

type RawMsg = i32;

impl Runtime {
    pub fn new(rd: Room, config: Config) -> Self {
        Self {
            _rd: rd,
            _config: config,
        }
    }

    pub fn process_msg(&self, msg: RawMsg) -> () {
        // match msg {
        //     Err(err) => MqttErrorHandler::handle_err(cli, err),
        //     Ok((channel, msg)) => handle_mess(cli, msg, channel),
        // }
    }
}

