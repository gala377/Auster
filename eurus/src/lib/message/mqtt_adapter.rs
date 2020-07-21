use std::error::Error;
use std::sync::mpsc;
use std::time::Duration;

use paho_mqtt as mqtt;

use crate::message;
use crate::room::RoomData;

//
// Private static nad const values
//
const CONN_RETRIES: usize = 12;
const RETRIE_WAIT_MS: u64 = 5000;

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
    pub fn new(rd: &RoomData) -> Result<Self, Box<dyn Error>> {
        let cli = create_mqtt_client(&rd)?;
        Ok(Self {
            cli,
            rd: rd.clone(),
        })
    }

    pub fn try_reconnect(&mut self) -> bool {
        println!("Connection lost trying to reconnect");
        for _ in 0..CONN_RETRIES {
            std::thread::sleep(Duration::from_millis(RETRIE_WAIT_MS));
            if self.cli.reconnect().is_ok() {
                println!("Successfully reconnected");
                return true;
            }
        }
        println!("Unable to reconnnect after several attempts");
        false
    }
}

impl message::Client for MqttClient {
    type Iter = MqttRecvChannel;

    fn connect(&mut self) -> Result<Self::Iter, Box<dyn Error>> {
        let recv = connect_mqtt_client(&mut self.cli, &self.rd)?;
        let iter = MqttRecvChannel(recv.into_iter());
        Ok(iter)
    }

    fn is_connected(&self) -> bool {
        return self.cli.is_connected();
    }

    fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        self.cli.disconnect(None)?;
        Ok(())
    }

    fn subscribe(&mut self, channels: Vec<String>) -> Result<(), Box<dyn Error>> {
        let qos: Vec<i32> = vec![2; channels.len()];
        match self.cli.subscribe_many(&channels, &qos) {
            Ok(qosv) => println!("QoS granted: {:?}", qosv),
            Err(e) => {
                println!("Error subscribing to topics {:?}", e);
                println!("Disconnecting");
                self.cli.disconnect(None).unwrap();
                return Err(Box::new(e));
            }
        }
        Ok(())
    }

    fn publish(&mut self, channel: String, msg: message::PubMsg) -> Result<(), Box<dyn Error>> {
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
    type Item = Option<message::SubMsg>;

    fn next(&mut self) -> Option<Self::Item> {
        println!("Next is called");
        match self.0.next() {
            None => None,
            Some(val) => match val {
                Some(val) => {
                    let msg: message::SubMsg =
                        match serde_json::from_str(val.payload_str().as_ref()) {
                            Ok(v) => {
                                println!("We have a msg: {:?}", v);
                                v
                            }
                            Err(err) => {
                                println!("Error while receiving message {}", err);
                                return Some(None);
                            }
                        };
                    println!("Message propertly decoded {:?}", msg);
                    Some(Some(msg))
                }
                None => {
                    println!("Channel returned `None`");
                    Some(None)
                }
            },
        }
    }
}

//
// Helper functions on pahu_mqtt api
//

fn create_mqtt_client(rd: &RoomData) -> Result<mqtt::Client, Box<dyn Error>> {
    let host = "tcp://localhost:1883"; // todo get from config
    let create_opts = get_creation_mqtt_options(host, rd);
    let cli = mqtt::Client::new(create_opts)?;
    Ok(cli)
}

fn connect_mqtt_client(
    cli: &mut mqtt::Client,
    rd: &RoomData,
) -> Result<PahuRecvChannel, Box<dyn Error>> {
    let rx = cli.start_consuming();
    let lwt = get_lwt_mess(&rd);
    let conn_opts = get_connection_options(lwt);
    match cli.connect(conn_opts) {
        Ok(_) => Ok(rx),
        Err(e) => Err(Box::new(e)),
    }
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
