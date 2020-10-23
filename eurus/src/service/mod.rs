use std::time::Duration;

use log::{debug, error, info};
use thiserror::Error;

use paho_mqtt as mqtt;
use mqtt::errors::MqttError;
use std::future::Future;

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

async fn start_room_rt(rd: Room, config: Config) -> Result<tokio::task::JoinHandle<()>> {
    let mut cli = get_mqtt_client(&rd, &config).await?;
    let msg_stream = cli.get_stream(25); // arbitrarily chosen
    connect_to_mqtt(&mut cli, &rd, &config).await?;
    subscribe(&mut cli, &rd, &config).await?;
    let handle = tokio::spawn(async move {
        info!("[rd-rt-{}] Created new room", rd.id);
        debug!("[rd-rt-{}] Waiting for messages", rd.id);
        let rd_id = rd.id;
        // hardcoded should be extracted to function
        let runtime = room::runtime::Runtime::new(rd, config);
        let channel_prefix = &config.runtime.room_channel_prefix;
        let msg = mqtt::MessageBuilder::new()
            .topic(format!("{}/{}/rt/read", channel_prefix, rd.id))
            .payload(serde_json::to_string(&message::Response::RuntimeStarted).unwrap())
            .qos(2)
            .finalize();
        cli.publish(msg).await.unwrap();
        // for msg in listener {
        //     info!("[rd-rt-{}] Got msg", rd_id);
            // match runtime.process_msg(msg) {
            //     ErrorHandling::Abort => {
            //         if cli.is_connected() {
            //             info!("[rd-rt-{}] Disconnecting", rd_id);
            //             // todo: unsubscribe from topics here
            //             cli.disconnect().unwrap();
            //         }
            //         break;
            //     }
            //     _ => (),
            // }
        // }
    });
    Ok(handle)
}

async fn get_mqtt_client(rd: &Room, config: &Config) -> Result<mqtt::AsyncClient> {
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(&config.mqtt.host)
        .client_id(format!("room-rt-{}-{}", rd.id, rd.pass))
        .mqtt_version(mqtt::MQTT_VERSION_5)
        .finalize();
    let cli = mqtt::AsyncClient::new(create_opts)?;
    let lwt = mqtt::MessageBuilder::new()
        .topic(format!("test/room/{}", rd.id))
        .payload(format!("Room rt {} lost connection", rd.id))
        .finalize();
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

async fn subscribe(cli: &mut mqtt::AsyncClient, rd: &Room, config: &Config) -> Result<()> {
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