mod prelude {
	pub use std::result::Result as StdResult;

	pub use anyhow::Result;
}
use prelude::*;

mod hap;
mod mqtt;
use mqtt::MqttMessage;

// crates.io
use ::hap::serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::fmt;

type OneshotTx = oneshot::Sender<Value>;
type OneshotRx = oneshot::Receiver<Value>;
type Tx = mpsc::Sender<MqttMessage>;
type Rx = mpsc::Receiver<MqttMessage>;

#[tokio::main]
async fn main() -> Result<()> {
	fmt::init();

	let (hap_tx, mqtt_rx) = mpsc::channel(8);
	let (mqtt_tx, hap_rx) = oneshot::channel();

	// select
	let _ = tokio::join!(hap::start(hap_tx, hap_rx), mqtt::start(mqtt_tx, mqtt_rx));

	Ok(())
}
