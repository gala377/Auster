use crate::config::Config;
use mongodb;
use mongodb::options::{ClientOptions, Credential};
use tracing::debug;

pub struct Connection {
    pub cli: mongodb::Client,
    pub db: mongodb::Database,
    pub users_col: mongodb::Collection,
    pub rooms_col: mongodb::Collection,
}

impl Connection {
    #[tracing::instrument(skip(config))]
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let mut cli_opt = ClientOptions::parse(&config.db.host).await?;
        cli_opt.credential = Some(
            Credential::builder()
                .username(Some(config.db.user.clone()))
                .password(Some(config.db.password.clone()))
                .source(Some(config.db.database.clone()))
                .mechanism(Some(mongodb::options::AuthMechanism::ScramSha1))
                .build(),
        );
        cli_opt.app_name = Some("eurus".into());
        let cli = mongodb::Client::with_options(cli_opt)?;
        debug!("Everything done... let's test this baby");
        // for db_name in cli.list_database_names(None, None).await? {
        //     debug!("{}", db_name);
        // }
        let db = cli.database(&config.db.database);
        debug!("So your name is: {}", db.name());
        for col in db.list_collection_names(None).await? {
            debug!("{}", col);
        }
        let users_col = db.collection(&config.db.users_collection);
        let rooms_col = db.collection(&config.db.rooms_collection);
        Ok(Self {
            cli,
            db,
            users_col,
            rooms_col,
        })
    }
}
