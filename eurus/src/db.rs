use crate::config::Config;
use mongodb;
use mongodb::options::{ClientOptions, Credential};

pub struct Connection {
    pub cli: mongodb::Client,
    pub db: mongodb::Database,
    pub users_col: mongodb::Collection,
}

impl Connection {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let mut cli_opt = ClientOptions::parse(&config.db.host).await?;
        cli_opt.credential = Some(
            Credential::builder()
                .username(config.db.user.clone())
                .password(config.db.password.clone())
                .build(),
        );
        cli_opt.app_name = Some("eurus".into());
        let cli = mongodb::Client::with_options(cli_opt)?;
        let db = cli.database(&config.db.database);
        let users_col = db.collection(&config.db.users_collection);
        Ok(Self { cli, db, users_col })
    }
}
