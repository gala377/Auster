use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum SubMsg {
    Hello,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PubMsg {
    Hey,
}
