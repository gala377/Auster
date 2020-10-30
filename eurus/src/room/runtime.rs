use tracing::info;

use crate::room::model::Room;
use crate::{config::Config, message, service};

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

    pub(crate) async fn process_msg(
        &self,
        player: &str,
        msg: message::Request,
    ) -> service::Command {
        match player {
            "rt" => info!("global msg {:?}", msg),
            player => info!("msg from player {}: {:?}", player, msg),
        }
        service::Command::Skip
    }
}
