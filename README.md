# mqtt-sse-bridge

A simple application that subscribes to an MQTT broker and forwards any messages to an [SSE](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events) endpoint.

## Usage

Create a configuration file `Config.toml` and start with `cargo run`

```toml
# Example configuration

[sse]
ip = "127.0.0.1"
port = 3030
endpoint = "events"

[mqtt]
client_id = "mqtt-sse-bridge"
host = "test.mosquitto.org"
port = 1883
username = "wildcard"
password = ""
topic = "test/#"
```

If started with `RUST_LOG=trace` environment variable set, all received MQTT messages are logged to standard output.
