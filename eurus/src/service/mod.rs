use std::{time::Duration, pin::Pin};

use log::{debug, error, info, warn};
use thiserror::Error;
use futures::StreamExt;
use paho_mqtt as mqtt;
use paho_mqtt::Error as MqttError;
use futures::Stream;
use crate::{
    config::Config,
    message,
    room::{self, RoomsRepository},
    room::{model::Room, RoomEntry},
};

pub mod dto;

type Result<T> = std::result::Result<T, RoomCreationError>;

#[derive(Error, Debug)]
pub enum RoomCreationError {
    #[error("{0}")]
    MqttConnectionError(#[from] MqttError),
    #[error("could not encode the message {0}")]
    MsgEncodingError(#[from] serde_json::Error),
    #[error("connection was reset")]
    ConnectionReset,
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("connection was reset")]
    ConnectionReset,
    #[error("could not decode message {0}")]
    MsgDecodingError(#[from] serde_json::Error),
}

pub(crate) enum Command {
    Skip,
    Abort(Option<String>),
    Response(message::Response),
}

pub async fn create_new_room(
    rep: &mut RoomsRepository,
    config: Config,
    room_req: dto::NewRoomReq,
) -> Result<dto::NewRoomResp> {
    let rd = rep.create_room(room_req.players_limit, room_req.rounds_limit);
    let re = RoomEntry::from(&rd);
    let resp = dto::NewRoomResp::from(&rd);
    if let Err(err) = start_room_rt(rd, config).await {
        rep.remove(re);
        return Err(err);
    }
    Ok(resp)
}

async fn start_room_rt(rd: Room, config: Config) -> Result<()> {
    let mut cli = get_mqtt_client(&rd, &config).await?;
    let msg_stream = cli.get_stream(25); // arbitrarily chosen
    connect_to_mqtt(&mut cli, &rd, &config).await?;
    subscribe_default(&mut cli, &rd, &config).await?;
    send_rt_start_msg(&mut cli, &rd, &config).await?;
    tokio::spawn(create_room_rt_task(cli, Box::pin(msg_stream), rd, config));
    Ok(())
}

async fn get_mqtt_client(rd: &Room, config: &Config) -> Result<mqtt::AsyncClient> {
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(&config.mqtt.host)
        .client_id(format!("room-rt-{}-{}", rd.id, rd.pass))
        .mqtt_version(mqtt::MQTT_VERSION_5)
        .finalize();
    let cli = mqtt::AsyncClient::new(create_opts)?;
    Ok(cli)
}

async fn connect_to_mqtt(cli: &mut mqtt::AsyncClient, rd: &Room, config: &Config) -> Result<()> {
    let lwt = mqtt::MessageBuilder::new()
        .topic(format!("test/room/{}", rd.id))
        .payload(format!("Room rt {} lost connection", rd.id))
        .finalize();
    // todo: get duration from configuration
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .clean_session(true) // todo: maybe we should start with clean for the test msg
        .will_message(lwt)
        .finalize();
    cli.connect(conn_opts).await?;
    Ok(())
}

async fn subscribe_default(cli: &mut mqtt::AsyncClient, rd: &Room, config: &Config) -> Result<()> {
    let channel_prefix = &config.runtime.room_channel_prefix;
    let channels = vec![format!("{}/{}/rt/write", channel_prefix, rd.id)];
    let qos: Vec<i32> = vec![2; channels.len()];
    match cli.subscribe_many(&channels, &qos).await {
        Ok(qosv) => debug!("QoS granted: {:?}", qosv),
        Err(e) => {
            error!("Error subscribing to topics {:?}", e);
            debug!("Disconnecting");
            cli.disconnect(None).await?;
            return Err(RoomCreationError::from(e));
        }
    }
    Ok(())
}

async fn send_rt_start_msg(cli: &mut mqtt::AsyncClient, rd: &Room, config: &Config) -> Result<()> {
    let channel_prefix = &config.runtime.room_channel_prefix;
    let msg = mqtt::MessageBuilder::new()
        .topic(format!("{}/{}/rt/read", channel_prefix, rd.id))
        .payload(serde_json::to_string(&message::Response::RuntimeStarted).unwrap())
        .qos(2)
        .finalize();
    cli.publish(msg).await?;
    Ok(())
}

type Topic = String;

fn parse_msg(msg: Option<mqtt::Message>) -> std::result::Result<(Topic, message::Request), RuntimeError> {
    match msg {
        Some(val) => Ok((
            val.topic().into(),
            serde_json::from_str(val.payload_str().as_ref())?,
        )),
        None => Err(RuntimeError::ConnectionReset),
    }

}

async fn create_room_rt_task<S>(mut cli: mqtt::AsyncClient, mut msg_stream: Pin<Box<S>>, rd: Room, config: Config) -> impl std::future::Future<Output=()>
where
    S: Stream<Item=Option<mqtt::Message>>,
{
    async move {
        let rd_id = rd.id;
        info!("[rd-rt-{}] Created new room", rd_id);
        debug!("[rd-rt-{}] Waiting for messages", rd_id);
        let runtime = room::runtime::Runtime::new(rd, config.clone());
        while let Some(msg) = msg_stream.next().await {
            info!("[rd-rt-{}] Got msg", &rd_id);
            let msg = parse_msg(msg);
            match msg {
                Ok((topic, msg)) => {
                    let resp = runtime.process_msg(msg).await;
                    if handle_resp(&mut cli, rd_id, topic, &config, resp).await {
                        break;
                    }
                }
                Err(RuntimeError::ConnectionReset) => {
                    if cli.is_connected() || !try_reconnect(&mut cli).await {
                        warn!("[rd-rt-{}] channel died", rd_id);
                        error!("[rd-rt-{}] aborting...", rd_id);
                        break;
                    }
                }
                Err(RuntimeError::MsgDecodingError(inner)) => {
                    // We don't know who send it so yeah
                    // just skip
                    error!("[rd-rt-{}] {}", rd_id, inner);
                }
            }
        }
    }
}

async fn handle_resp(cli: &mut mqtt::AsyncClient, rd_id: usize, src_topic: Topic, config: &Config, cmd: Command) -> bool {
    match cmd {
        Command::Abort(msg) => {
            if let Some(msg) = msg {
                error!("[rd-rt-{}] Aborting with message {}", rd_id, msg);
            } else {
                error!("[rd-rt-{}] Aborting...", rd_id);
            }
            if cli.is_connected() {
                info!("[rd-rt-{}] Disconnecting", rd_id);
                // todo: unsubscribe from topics here
                cli.disconnect(None).await.unwrap();
            }
            true
        }
        Command::Response(message::Response::Priv(player, resp)) => {
            send_resp(&player.to_string(), resp.as_ref(), cli, &config, rd_id).await;
            false
        }
        Command::Response(resp) => {
            send_resp("rt", &resp, cli, config, rd_id).await;
            false
        }
        _ => false,
    }
}

const CONN_RETRIES: u32 = 12;
const RETRY_WAIT_MS: u64 = 5000;

async fn try_reconnect(cli: &mut mqtt::AsyncClient) -> bool {
    warn!("Connection lost trying to reconnect");
    for _ in 0..CONN_RETRIES {
        if cli.reconnect().await.is_ok() {
            info!("Successfully reconnected");
            return true;
        }
        tokio::time::delay_for(Duration::from_millis(RETRY_WAIT_MS)).await;
    }
    error!("Unable to reconnect after several attempts");
    false
}

async fn send_resp(to: &str, resp: &message::Response, cli: &mut mqtt::AsyncClient, config: &Config, rd_id: usize) {
    let channel_prefix = &config.runtime.room_channel_prefix;
    let msg = mqtt::MessageBuilder::new()
        .topic(format!("{}/{}/rt/read", channel_prefix, rd_id))
        .payload(serde_json::to_string(&resp).unwrap())
        .qos(2)
        .finalize();
    cli.publish(msg).await.unwrap();
}