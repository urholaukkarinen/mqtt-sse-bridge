#![deny(clippy::all)]

use std::convert::Infallible;
use std::error::Error;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use rumqttc::Event::Incoming;
use rumqttc::Packet::Publish;
use rumqttc::{AsyncClient, EventLoop, MqttOptions};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use warp::{sse::Event, Filter, Reply};

use crate::config::{Config, MqttConfig};

mod config;

const CONFIG_FILENAME: &str = "Config.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let config = read_config(CONFIG_FILENAME).await?;

    let streams = Arc::new(Mutex::new(Vec::new()));

    println!("{}", config.mqtt);

    start_mqtt_subscription(config.mqtt, streams.clone());

    let routes = warp::path(config.sse.endpoint.clone())
        .and(warp::get())
        .map(move || sse_reply(&streams))
        .with(warp::cors().allow_any_origin());

    let ip = IpAddr::from_str(&config.sse.ip)
        .map_err(|err| anyhow!("Failed to parse IP address: {}", err))?;

    println!(
        "SSE endpoint created: http://{}:{}/{}",
        ip, config.sse.port, config.sse.endpoint
    );

    warp::serve(routes).run((ip, config.sse.port)).await;

    Ok(())
}

async fn read_config(path: impl AsRef<Path>) -> Result<Config, Box<dyn Error>> {
    let config = tokio::fs::read_to_string(path)
        .await
        .map_err(|err| anyhow!("Could not read Config.toml: {}", err))?;

    let config =
        toml::from_str(&config).map_err(|err| anyhow!("Failed to parse Config.toml: {}", err))?;

    Ok(config)
}

fn sse_reply(streams: &Arc<Mutex<Vec<UnboundedSender<String>>>>) -> impl Reply {
    let (tx, rx) = unbounded_channel();
    streams.lock().unwrap().push(tx);
    let stream = UnboundedReceiverStream::new(rx).map(make_event);
    warp::sse::reply(warp::sse::keep_alive().stream(stream))
}

fn make_event(data: String) -> Result<Event, Infallible> {
    Ok(Event::default().data(data))
}

fn start_mqtt_subscription(config: MqttConfig, streams: Arc<Mutex<Vec<UnboundedSender<String>>>>) {
    tokio::spawn(async move {
        loop {
            let (_mqtt_client, mut mqtt_event_loop) = setup_mqtt(config.clone()).await;

            loop {
                match mqtt_event_loop.poll().await {
                    Ok(Incoming(Publish(rumqttc::Publish { payload, .. }))) => {
                        let data = String::from_utf8_lossy(&payload).to_string();
                        log::trace!("{}", data);

                        streams
                            .lock()
                            .unwrap()
                            .retain(|tx| tx.send(data.clone()).is_ok());
                    }
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("{:?}", err);

                        break;
                    }
                }
            }
        }
    });
}

async fn setup_mqtt(config: MqttConfig) -> (AsyncClient, EventLoop) {
    let mut mqtt_opts = MqttOptions::new(config.client_id, config.host, config.port);
    if let Some((username, password)) = config.username.zip(config.password) {
        mqtt_opts.set_credentials(username, password);
    }

    let (client, event_loop) = AsyncClient::new(mqtt_opts, 1);

    client
        .subscribe(config.topic, rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    (client, event_loop)
}
