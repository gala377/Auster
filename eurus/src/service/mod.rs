use std::{pin::Pin, time::Duration};

use crate::{
    config::Config,
    message,
    repository::{self, DataRepository, RepReq, RepReqChannel, RepResp, RoomEntry},
    room,
    room::model::Room,
};
use futures::Stream;
use futures::StreamExt;
use paho_mqtt as mqtt;
use paho_mqtt::Error as MqttError;
use thiserror::Error;
use tracing::Instrument;
use tracing::{debug, error, info, warn};

pub mod dto;

type Result<T> = std::result::Result<T, RoomCreationError>;
type Topic = String;

#[derive(Error, Debug)]
pub enum RoomCreationError {
    #[error("error: {0}")]
    UnknownError(String),
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

#[allow(dead_code)]
pub(crate) enum Command {
    Skip,
    Abort(Option<String>),
    Response(message::Response),
}

struct RoomData {
    pub entry: RoomEntry,
    pub players_limit: usize,
    pub rounds_limit: usize,
    id_as_base64: String,
}

impl RoomData {
    pub(super) fn internal_id(&self) -> InternalRoomId {
        InternalRoomId::new(self.entry.id.clone(), self.id_as_base64.clone())
    }
}

impl std::fmt::Display for RoomData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Room({})", self.id_as_base64)
    }
}

impl Into<Room> for RoomData {
    fn into(self) -> Room {
        Room::new(
            self.entry.id,
            self.entry.password,
            self.players_limit,
            self.rounds_limit,
        )
    }
}
#[derive(Clone)]
struct InternalRoomId {
    pub id: repository::EntryId,
    as_base64: String,
}

impl InternalRoomId {
    pub fn new(id: repository::EntryId, as_base64: String) -> Self {
        Self { id, as_base64 }
    }
}

impl std::fmt::Display for InternalRoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Room({})", self.as_base64)
    }
}

static ROOM_CHANNEL_PREFIX: &str = "rooms";

#[tracing::instrument(skip(rep))]
pub async fn create_new_room(
    mut rep: RepReqChannel,
    config: Config,
    room_req: dto::NewRoomReq,
) -> Result<dto::NewRoomResp> {
    let re = DataRepository::send_req(
        &mut rep,
        RepReq::CreateRoom {
            players_limit: room_req.players_limit,
        },
    )
    .await;
    let re = match re {
        Ok(RepResp::RoomCreated(val)) => val,
        _ => {
            return Err(RoomCreationError::UnknownError(
                "couldn't complete creation request in room repository".to_owned(),
            ))
        }
    };
    let resp = dto::NewRoomResp::from(&re);
    let room_id = re.id;
    let id_as_base64 = base64::encode(&re.id);
    let rd = RoomData {
        entry: re,
        players_limit: room_req.players_limit,
        rounds_limit: room_req.rounds_limit,
        id_as_base64,
    };
    if let Err(err) = start_room_rt(rd, config).await {
        // todo: some error handling?
        // for now we don't care
        let _ = DataRepository::send_req(&mut rep, RepReq::RemoveRoom { room_id }).await;
        return Err(err);
    }
    Ok(resp)
}

#[tracing::instrument(skip(rd, config))]
async fn start_room_rt(rd: RoomData, config: Config) -> Result<()> {
    let mut cli = get_mqtt_client(&rd.id_as_base64, &config).await?;
    let msg_stream = cli.get_stream(25); // arbitrarily chosen
    connect_to_mqtt(&mut cli, &rd.id_as_base64).await?;
    subscribe_default(&mut cli, &rd.id_as_base64, rd.players_limit).await?;
    send_rt_start_msg(&mut cli, &rd.id_as_base64).await?;
    info!("spawning room rt");
    tokio::spawn(create_room_rt_task(cli, Box::pin(msg_stream), rd, config).await);
    info!("spawned");
    Ok(())
}

#[tracing::instrument(skip(config))]
async fn get_mqtt_client(room_id: &str, config: &Config) -> Result<mqtt::AsyncClient> {
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(&config.mqtt.host)
        .client_id(format!("room-rt-{}", room_id))
        .mqtt_version(mqtt::MQTT_VERSION_5)
        .finalize();
    let cli = mqtt::AsyncClient::new(create_opts)?;
    Ok(cli)
}

#[tracing::instrument(skip(cli))]
async fn connect_to_mqtt(cli: &mut mqtt::AsyncClient, room_id: &str) -> Result<()> {
    let lwt = mqtt::MessageBuilder::new()
        .topic(format!("test/room/{}", room_id))
        .payload(format!("Room rt {} lost connection", room_id))
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

#[tracing::instrument(skip(cli))]
async fn subscribe_default(
    cli: &mut mqtt::AsyncClient,
    room_id: &str,
    players_limit: usize,
) -> Result<()> {
    let mut channels = vec![format!("{}/{}/rt/write", ROOM_CHANNEL_PREFIX, room_id)];
    for i in 0..players_limit {
        channels.push(format!("{}/{}/{}/write", ROOM_CHANNEL_PREFIX, room_id, i));
    }
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

#[tracing::instrument(skip(cli, msg_stream, rd, config))]
async fn create_room_rt_task<S>(
    mut cli: mqtt::AsyncClient,
    mut msg_stream: Pin<Box<S>>,
    rd: RoomData,
    config: Config,
) -> impl std::future::Future<Output = ()>
where
    S: Stream<Item = Option<mqtt::Message>>,
{
    info!("Inside a room creation task");
    let span = tracing::debug_span!("room message handling", room_id = rd.id_as_base64.as_str());
    async move {
        let room_id = rd.internal_id();
        info!("Created new room");
        debug!("Waiting for messages");
        let runtime = room::runtime::Runtime::new(rd.into(), config.clone());
        info!("Runtime created");
        while let Some(msg) = msg_stream.next().await {
            debug!("Got msg");
            let msg = parse_msg(msg);
            match msg {
                Ok((topic, msg)) => {
                    let resp = runtime.process_msg(&player_from_topic(&topic), msg).await;
                    if handle_resp(&mut cli, &room_id, topic, resp).await {
                        break;
                    }
                }
                Err(RuntimeError::ConnectionReset) => {
                    if cli.is_connected() || !try_reconnect(&mut cli).await {
                        warn!("channel died");
                        info!("aborting...");
                        break;
                    }
                }
                Err(RuntimeError::MsgDecodingError(inner)) => {
                    // We don't know who send it so yeah
                    // just skip
                    error!("{}", inner);
                }
            }
        }
    }
    .instrument(span)
}

#[tracing::instrument(skip(cli))]
async fn send_rt_start_msg(cli: &mut mqtt::AsyncClient, room_id: &str) -> Result<()> {
    let msg = mqtt::MessageBuilder::new()
        .topic(format!("{}/{}/rt/read", ROOM_CHANNEL_PREFIX, room_id))
        .payload(serde_json::to_string(&message::Response::RuntimeStarted).unwrap())
        .qos(2)
        .finalize();
    cli.publish(msg).await?;
    Ok(())
}

#[tracing::instrument(skip(msg))]
fn parse_msg(
    msg: Option<mqtt::Message>,
) -> std::result::Result<(Topic, message::Request), RuntimeError> {
    match msg {
        Some(val) => Ok((
            val.topic().into(),
            serde_json::from_str(val.payload_str().as_ref())?,
        )),
        None => Err(RuntimeError::ConnectionReset),
    }
}

#[tracing::instrument(skip(cli, rd_id, cmd))]
async fn handle_resp(
    cli: &mut mqtt::AsyncClient,
    rd_id: &InternalRoomId,
    src_topic: Topic,
    cmd: Command,
) -> bool {
    match cmd {
        Command::Abort(msg) => {
            if let Some(msg) = msg {
                error!("Aborting with message {}", msg);
            } else {
                error!("Aborting...");
            }
            if cli.is_connected() {
                info!("Disconnecting");
                // todo: unsubscribe from topics here
                cli.disconnect(None).await.unwrap();
            }
            true
        }
        Command::Response(message::Response::Priv(player, resp)) => {
            send_resp(&player.to_string(), resp.as_ref(), cli, &rd_id.as_base64).await;
            false
        }
        Command::Response(resp) => {
            send_resp("rt", &resp, cli, &rd_id.as_base64).await;
            false
        }
        _ => false,
    }
}

const CONN_RETRIES: u32 = 12;
const RETRY_WAIT_MS: u64 = 5000;

#[tracing::instrument(skip(cli))]
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

#[tracing::instrument(skip(cli))]
async fn send_resp(to: &str, resp: &message::Response, cli: &mut mqtt::AsyncClient, rd_id: &str) {
    let msg = mqtt::MessageBuilder::new()
        .topic(format!("{}/{}/rt/read", ROOM_CHANNEL_PREFIX, rd_id))
        .payload(serde_json::to_string(&resp).unwrap())
        .qos(2)
        .finalize();
    cli.publish(msg).await.unwrap();
}

fn player_from_topic(topic: &Topic) -> String {
    let user = topic.split('/').nth(2).unwrap();
    user.to_owned()
}
