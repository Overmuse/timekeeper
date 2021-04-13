use alpaca::{
    rest::clock::{Clock, GetClock},
    Client,
};
use anyhow::{anyhow, Result};
use dotenv::dotenv;
use rdkafka::producer::FutureRecord;
use serde::{Deserialize, Serialize};
use std::process::exit;

mod settings;
use settings::Settings;

#[derive(Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum State {
    Open { next_close: usize },
    Closed { next_open: usize },
}

impl From<Clock> for State {
    fn from(clock: Clock) -> State {
        if clock.is_open {
            State::Open {
                next_close: (clock.next_close - clock.timestamp).num_seconds() as usize,
            }
        } else {
            State::Closed {
                next_open: (clock.next_open - clock.timestamp).num_seconds() as usize,
            }
        }
    }
}

async fn run() -> Result<()> {
    let _ = dotenv();
    let settings = Settings::new()?;
    let producer = kafka_settings::producer(&settings.kafka)?;
    let client = Client::new(
        settings.alpaca.base_url,
        settings.alpaca.key_id,
        settings.alpaca.secret_key,
    )?;

    let clock = client.send(GetClock).await?;
    let state: State = clock.into();
    producer
        .send(
            FutureRecord::to("time")
                .key("time")
                .payload(&serde_json::to_string(&state)?),
            None,
        )
        .await
        .map(|_| ())
        .map_err(|(e, msg)| anyhow!("Error from Kafka: {:?}\n Message: {:?}", e, msg))
}

#[tokio::main]
async fn main() -> Result<()> {
    match run().await {
        Ok(_) => exit(0),
        Err(e) => {
            eprintln!("{:?}", e);
            exit(1)
        }
    }
}
