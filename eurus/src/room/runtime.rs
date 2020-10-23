use log::info;

use super::model::Room;
use crate::{
    config::Config,
    message,
    service::{self, dto},
};

pub struct Runtime {
    _rd: Room,
    _config: Config,
}


impl Runtime {
    pub fn new(rd: Room, config: Config) -> Self {
        Self {
            _rd: rd,
            _config: config,
        }
    }

    pub(crate) async fn process_msg(&self, msg: message::Request) -> service::Command {
        service::Command::Skip
    }
}

