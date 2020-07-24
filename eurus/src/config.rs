use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub mqtt: Mqtt,
    pub runtime: Runtime,
}

#[derive(Deserialize, Clone)]
pub struct Mqtt {
    pub host: String,
    pub user: String,
    pub password: String,
}

#[derive(Deserialize, Clone)]
pub struct Runtime {
    pub server_address: String,
    pub room_channel_prefix: String,
}
