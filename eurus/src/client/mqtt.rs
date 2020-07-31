use std::sync::mpsc;
use std::time::Duration;

use log::{debug, error, info, warn};
use thiserror::Error;

use mqtt::errors::MqttError as RawMqttError;
use paho_mqtt as mqtt;

use crate::{
    client::{Client, ErrorHandler, ErrorHandling},
    message,
    room::RoomData,
};

//
// Private static nad const values
//
const CONN_RETRIES: usize = 12;
const RETRY_WAIT_MS: u64 = 5000;

//
// Private Pahu type aliases
//
type PahuRecvIter = mpsc::IntoIter<Option<mqtt::Message>>;

type Result<T> = std::result::Result<T, MqttError>;

//
// Public wrapper interface, only necessary api
//
pub struct MqttClient {
    cli: mqtt::Client,
    rd: RoomData,
}

impl MqttClient {
    pub fn new(rd: &RoomData, host: &String) -> Result<Self> {
        let cli = create_mqtt_client(rd, host)?;
        Ok(Self {
            cli,
            rd: rd.clone(),
        })
    }

    pub fn try_reconnect(&mut self) -> bool {
        warn!("Connection lost trying to reconnect");
        for _ in 0..CONN_RETRIES {
            std::thread::sleep(Duration::from_millis(RETRY_WAIT_MS));
            if self.cli.reconnect().is_ok() {
                info!("Successfully reconnected");
                return true;
            }
        }
        error!("Unable to reconnnect after several attempts");
        false
    }
}

impl Client for MqttClient {
    type Iter = MqttRecvChannel;
    type ClientError = MqttError;

    fn connect(&mut self) -> Result<()> {
        connect_mqtt_client(&mut self.cli, &self.rd)
    }

    fn is_connected(&self) -> bool {
        return self.cli.is_connected();
    }

    fn disconnect(&mut self) -> Result<()> {
        self.cli.disconnect(None)?;
        Ok(())
    }

    fn iter_msg(&mut self) -> Self::Iter {
        let rx = self.cli.start_consuming();
        MqttRecvChannel(rx.into_iter())
    }

    fn subscribe(&mut self, channels: Vec<String>) -> Result<()> {
        let qos: Vec<i32> = vec![2; channels.len()];
        match self.cli.subscribe_many(&channels, &qos) {
            Ok(qosv) => debug!("QoS granted: {:?}", qosv),
            Err(e) => {
                error!("Error subscribing to topics {:?}", e);
                debug!("Disconnecting");
                self.cli.disconnect(None).unwrap();
                return Err(MqttError::from(e));
            }
        }
        Ok(())
    }

    fn publish(&mut self, channel: String, msg: message::PubMsg) -> Result<()> {
        let msg = serde_json::to_string(&msg)?;
        let msg = mqtt::MessageBuilder::new()
            .topic(channel)
            .payload(msg)
            .qos(2)
            .finalize();
        self.cli.publish(msg)?;
        Ok(())
    }
}

pub struct MqttRecvChannel(PahuRecvIter);

impl Iterator for MqttRecvChannel {
    type Item = Result<(String, message::SubMsg)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|val| parse_msg(val).map_err(MqttError::from))
    }
}

#[derive(Error, Debug)]
pub enum MqttError {
    #[error("connection error {0}")]
    ConnectionError(#[from] RawMqttError),
    #[error("couldn't decode incoming msg {0}")]
    MsgDecodingError(#[from] serde_json::Error),
    #[error("connection was reset")]
    ConnectionReset,
}

//
// Helper functions on pahu_mqtt api
//

fn create_mqtt_client(rd: &RoomData, host: impl Into<String>) -> Result<mqtt::Client> {
    let create_opts = get_creation_mqtt_options(rd, host);
    let cli = mqtt::Client::new(create_opts)?;
    Ok(cli)
}

fn connect_mqtt_client(cli: &mut mqtt::Client, rd: &RoomData) -> Result<()> {
    let lwt = get_lwt_mess(&rd);
    let conn_opts = get_connection_options(lwt);
    cli.connect(conn_opts)?;
    Ok(())
}

fn get_creation_mqtt_options(rd: &RoomData, host: impl Into<String>) -> mqtt::CreateOptions {
    mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id(format!("room-rt-{}-{}", rd.id, rd.pass))
        .mqtt_version(mqtt::MQTT_VERSION_5)
        .finalize()
}

fn get_lwt_mess(rd: &RoomData) -> mqtt::Message {
    mqtt::MessageBuilder::new()
        .topic(format!("test/room/{}", rd.id))
        .payload(format!("Room rt {} lost connection", rd.id))
        .finalize()
}

fn get_connection_options(lwt: mqtt::Message) -> mqtt::ConnectOptions {
    // todo: get duration from configuration
    mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .clean_session(true) // tood: maybe we should start with clean for the test msg
        .will_message(lwt)
        .finalize()
}

fn parse_msg(msg: Option<mqtt::Message>) -> Result<(String, message::SubMsg)> {
    match msg {
        Some(val) => Ok((
            val.topic().into(),
            serde_json::from_str(val.payload_str().as_ref())?,
        )),
        None => Err(MqttError::ConnectionReset),
    }
}

#[derive(Default)]
pub struct MqttErrorHandler;

impl ErrorHandler for MqttErrorHandler {
    type Client = MqttClient;

    fn handle_err(cli: &mut MqttClient, err: MqttError) -> ErrorHandling {
        match err {
            MqttError::MsgDecodingError(err) => {
                error!("[rd-rt-{}] {}", cli.rd.id, err);
                ErrorHandling::Skip
            }
            MqttError::ConnectionError(err) => handle_err_abort(cli, err),
            err @ MqttError::ConnectionReset => {
                if cli.is_connected() || !cli.try_reconnect() {
                    warn!("[rd-rt-{}] channel died", cli.rd.id);
                    handle_err_abort(cli, err)
                } else {
                    ErrorHandling::Skip
                }
            }
        }
    }
}

fn handle_err_abort(cli: &mut MqttClient, err: impl std::error::Error) -> ErrorHandling {
    error!("[rd-rt-{}] {}", cli.rd.id, err);
    ErrorHandling::Abort
}
