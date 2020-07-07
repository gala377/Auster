use std::error::Error;
use std::time::Duration;
use std::thread::JoinHandle;
use std::sync::mpsc;

use paho_mqtt as mqtt;

use crate::room::{RoomData, RoomsRepository};

pub enum RoomCreationError {
    MqqtConnectionError(Box<dyn Error>),
}

pub fn create_new_room(rep: &mut RoomsRepository) -> Result<RoomData, RoomCreationError> {
    let rd = rep.create_room();
    use RoomCreationError::*;
    if let Err(err @ MqqtConnectionError(_)) = start_room_rt(rd.clone()) {
        rep.remove(rd);
        return Err(err);
    }
    Ok(rd)
}

fn start_room_rt(rd: RoomData) -> Result<JoinHandle<()>, RoomCreationError> {
    let (mut cli, rx) = match create_mqtt_client(&rd) {
        Ok((cli, rx)) => (cli, rx),
        Err(err) => return Err(RoomCreationError::MqqtConnectionError(err)),
    };
    let handle = std::thread::spawn(move || {
        println!("[rd-rt-{}] Hey my room is {:?}", rd.id, rd);
        println!("[rd-rt-{}] Waiting for messages", rd.id);
        for msg in rx.iter() {
            println!("[rd-rt-{}] Got message", rd.id);
            if let Some(msg) = msg {
                handle_mess(&mut cli, msg);
            } else if cli.is_connected() || !try_reconnect(&cli) {
                println!("[rd-rt-{}] channel died", rd.id);
                // channel died
                break;
            }
        }
        if cli.is_connected() {
            println!("[rd-rt-{}] Disconnecting", rd.id);
            // todo: unsunscribe from topics here
            cli.disconnect(None).unwrap();
        }
    });
    Ok(handle)
}

fn try_reconnect(cli: &mqtt::Client) -> bool {
    println!("Connection lost trying to reconnect");
    const RETRIES: usize = 12;
    const WAIT_MS: u64 = 5000; 
    for _ in 0..RETRIES {
        std::thread::sleep(Duration::from_millis(WAIT_MS));
        if cli.reconnect().is_ok() {
            println!("Successfully reconnected");
            return true;
        }
    }
    println!("Unable to reconnnect after several attempts");
    false
}

fn handle_mess(_cli: &mut mqtt::Client, msg: mqtt::Message) {
    println!("Got msg: {:?}", msg);
}

type MqttClientConn = (mqtt::Client, mpsc::Receiver<Option<mqtt::Message>>);

fn create_mqtt_client(rd: &RoomData) -> Result<MqttClientConn, Box<dyn Error>> {
    let host = "tcp://localhost:1883"; // todo get from config
    let create_opts = get_creation_mqtt_options(host, rd);
    let mut cli = mqtt::Client::new(create_opts)?;
    let rx = cli.start_consuming();
    let lwt = get_lwt_mess(&rd);
    let conn_opts = get_connection_options(lwt);
    let subscriptions = [
        format!("test-{}", rd.id),
        format!("room-{}", rd.id),
    ];
    connect_mqtt_client(conn_opts, &mut cli, &subscriptions)?;   
    Ok((cli, rx))
}

fn connect_mqtt_client(conn_opts: mqtt::ConnectOptions, cli: &mut mqtt::Client, subs: &[String]) -> Result<(), Box<dyn Error>>{
    match cli.connect(conn_opts) {
        Ok(rsp) => {
            if let Some((server_uri, mqtt_ver, sess_present)) = rsp.connect_response() {
                println!("Connected to: {} with MQTT version {}", server_uri, mqtt_ver);
                if !sess_present {
                    // todo: Here we should subscribe to topics with requested QoS
                    // todo: And a vector of subscriptions
                    let qos: Vec<i32> = subs.iter().map(|_| 2).collect();
                    match cli.subscribe_many(subs, &qos) {
                        Ok(qosv) => println!("QoS granted: {:?}", qosv),
                        Err(e) => {
                            println!("Error subscribing to topics {:?}", e);
                            println!("Disconnecting");
                            cli.disconnect(None).unwrap();
                            return Err(Box::new(e));
                        }
                    }
                }
            }
            Ok(())
        },
        Err(e) => {
            println!("Error connecting to the broker {:?}", e);
            Err(Box::new(e))
        }
    }
}

fn get_creation_mqtt_options(host: impl AsRef<str>, rd: &RoomData) -> mqtt::CreateOptions {
    mqtt::CreateOptionsBuilder::new()
        .server_uri(host.as_ref())
        .client_id(format!("room-rt-{}", rd.id))
        .finalize()
}

fn get_lwt_mess(rd: &RoomData) -> mqtt::Message {
    mqtt::MessageBuilder::new()
        .topic(format!("test-{}", rd.id))
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