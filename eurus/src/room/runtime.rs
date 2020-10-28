use tracing::info;

use super::model::Room;
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
        player: Option<super::model::PlayerId>,
        msg: message::Request,
    ) -> service::Command {
        match player {
            Some(player) => info!("msg from player {}: {:?}", player, msg),
            None => info!("global msg {:?}", msg),
        }
        service::Command::Skip
    }
}
