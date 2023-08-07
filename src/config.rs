use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub sse: SseConfig,
    pub mqtt: MqttConfig,
}

#[derive(Serialize, Deserialize)]
pub struct SseConfig {
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_endpoint")]
    pub endpoint: String,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port(),
            endpoint: default_endpoint(),
        }
    }
}

fn default_ip() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3030
}

fn default_endpoint() -> String {
    "events".to_string()
}

#[derive(Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub topic: String,
}

impl Display for MqttConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"MQTT Configuration:
Client ID: {}
Host: {}
Port: {}
Topic: {}
Credentials: {}
"#,
            self.client_id,
            self.host,
            self.port,
            self.topic,
            match self.username.as_ref().zip(self.password.as_ref()) {
                Some(_) => "provided",
                None => "none",
            }
        )
    }
}
