use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub mqtt: Mqtt,
    pub db: Db,
    pub runtime: Runtime,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Mqtt {
    pub host: String,
    pub user: String,
    pub password: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Db {
    pub host: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub users_collection: String,
    pub rooms_collection: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Runtime {
    pub server_address: String,
}
