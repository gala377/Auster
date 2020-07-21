use serde::{Deserialize, Serialize};
use std::error::Error;

pub mod mqtt_adapter;

#[derive(Debug, Serialize, Deserialize)]
pub enum SubMsg {
    Hello,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PubMsg {
    Hey,
}

pub trait Client {
    /// Iterator type returned from the connect method.
    /// `next` method should block waiting for the next message.
    type Iter: Iterator;

    /// Returns blocking iterator over
    /// messages from the subscribed channels.
    fn connect(&mut self) -> Result<Self::Iter, Box<dyn Error>>;

    /// Resturs bool of the connection is still alive.
    fn is_connected(&self) -> bool;

    /// Publish message in to the specified channel.
    fn publish(&mut self, channel: String, msg: PubMsg) -> Result<(), Box<dyn Error>>;

    /// Subscribe to the specified channels.
    /// Messages should be received from the iterator
    /// obtained from the `connect` message.
    fn subscribe(&mut self, channels: Vec<String>) -> Result<(), Box<dyn Error>>;

    /// Disconnects the client.
    fn disconnect(&mut self) -> Result<(), Box<dyn Error>>;
}
