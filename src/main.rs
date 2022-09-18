use std::convert::Infallible;
use std::error::Error;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;

use anyhow::anyhow;
use rumqttc::Event::Incoming;
use rumqttc::Packet::Publish;
use rumqttc::{AsyncClient, EventLoop, MqttOptions};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use warp::{sse::Event, Filter, Reply};

use crate::config::{Config, MqttConfig};

mod config;

const CONFIG_FILENAME: &str = "Config.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let config = read_config(CONFIG_FILENAME).await?;

    let (tx, _rx) = channel::<String>(config.sse.buffer_size);

    println!("{}", config.mqtt);

    start_mqtt_subscription(config.mqtt, tx.clone());

    let routes = warp::path(config.sse.endpoint.clone())
        .and(warp::get())
        .map(move || sse_reply(tx.subscribe()))
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

fn sse_reply(receiver: Receiver<String>) -> impl Reply {
    let stream = BroadcastStream::new(receiver).filter_map(|val| val.ok().map(make_event));
    warp::sse::reply(warp::sse::keep_alive().stream(stream))
}

fn make_event(data: String) -> Result<Event, Infallible> {
    Ok(Event::default().data(data))
}

fn start_mqtt_subscription(config: MqttConfig, sender: Sender<String>) {
    tokio::spawn(async move {
        let (_mqtt_client, mut mqtt_event_loop) = setup_mqtt(config).await;

        loop {
            match mqtt_event_loop.poll().await {
                Ok(Incoming(Publish(rumqttc::Publish { payload, .. }))) => {
                    let data = String::from_utf8_lossy(&payload).to_string();
                    log::trace!("{}", data);
                    sender.send(data).ok();
                }
                Ok(_) => {}
                Err(err) => {
                    log::error!("{:?}", err);
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
