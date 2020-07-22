use std::sync::mpsc;
use std::time::Duration;

use thiserror::Error;
use log::{info, error, debug, warn};

use mqtt::errors::MqttError as RawMqttError;
use paho_mqtt as mqtt;

use crate::message::{self, Client, ErrorHandler, ErrorHandling};
use crate::room::RoomData;

//
// Private static nad const values
//
const CONN_RETRIES: usize = 12;
const RETRY_WAIT_MS: u64 = 5000;

//
// Private Pahu type aliases
//
type PahuRecvChannel = mpsc::Receiver<Option<mqtt::Message>>;
type PahuRecvIter = mpsc::IntoIter<Option<mqtt::Message>>;

//
// Public wrapper interface, only necessary api
//
pub struct MqttClient {
    cli: mqtt::Client,
    rd: RoomData,
}

impl MqttClient {
    pub fn new(rd: &RoomData) -> Result<Self, MqttError> {
        let cli = create_mqtt_client(&rd)?;
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

impl message::Client for MqttClient {
    type Iter = MqttRecvChannel;
    type ClientError = MqttError;

    fn connect(&mut self) -> Result<Self::Iter, MqttError> {
        let recv = connect_mqtt_client(&mut self.cli, &self.rd)?;
        let iter = MqttRecvChannel(recv.into_iter());
        Ok(iter)
    }

    fn is_connected(&self) -> bool {
        return self.cli.is_connected();
    }

    fn disconnect(&mut self) -> Result<(), MqttError> {
        self.cli.disconnect(None)?;
        Ok(())
    }

    fn subscribe(&mut self, channels: Vec<String>) -> Result<(), MqttError> {
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

    fn publish(&mut self, channel: String, msg: message::PubMsg) -> Result<(), MqttError> {
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
    // todo: Change it to Result and handle cases aproprietly
    type Item = Result<message::SubMsg, MqttError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|val| handle_msg(val).map_err(MqttError::from))
    }
}

#[derive(Error, Debug)]
pub enum MqttError {
    #[error("connection error: `{0}`")]
    ConnectionError(#[from] RawMqttError),
    #[error("couldn't decode incoming msg: `{0}`")]
    MsgDecodingError(#[from] serde_json::Error),
    #[error("connection was reset")]
    ConnectionReset,
}

//
// Helper functions on pahu_mqtt api
//

fn create_mqtt_client(rd: &RoomData) -> Result<mqtt::Client, MqttError> {
    let host = "tcp://localhost:1883"; // todo get from config
    let create_opts = get_creation_mqtt_options(host, rd);
    let cli = mqtt::Client::new(create_opts)?;
    Ok(cli)
}

fn connect_mqtt_client(
    cli: &mut mqtt::Client,
    rd: &RoomData,
) -> Result<PahuRecvChannel, MqttError> {
    let rx = cli.start_consuming();
    let lwt = get_lwt_mess(&rd);
    let conn_opts = get_connection_options(lwt);
    Ok(cli.connect(conn_opts).map(move |_| rx)?)
}

fn get_creation_mqtt_options(host: impl AsRef<str>, rd: &RoomData) -> mqtt::CreateOptions {
    mqtt::CreateOptionsBuilder::new()
        .server_uri(host.as_ref())
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

fn handle_msg(msg: Option<mqtt::Message>) -> Result<message::SubMsg, MqttError> {
    match msg {
        Some(val) => Ok(serde_json::from_str(val.payload_str().as_ref())?),
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
