# Factory+ Rust Service Client

## Overview

This library provides a client for interacting with [Factory+](https://factoryplus.app.amrc.co.uk/) services in Rust
applications.

## Installation and Usage

- Add this crate to your `cargo.toml`:

```bash
cargo add rs_service_client
```

- Import the `ServiceClient`:

```rust
use rs_service_client::service::ServiceClient;
```

- Interact with it as an entry point.

## Example

This example creates a `ServiceClient`, subscribes to all messages on a Sparkplug device topic, and prints the received
data. Here we depend on the [tokio](https://crates.io/crates/tokio) async runtime. Note that we can iterate on the
receiver to pull the data when it's available, and the Sparkplug payloads are decoded for us.

```rust
use rs_service_client::service::mqtt::protocol::MqttProtocol;
use rs_service_client::service::ServiceClient;

#[tokio::main]
async fn main() {
    let service_client = ServiceClient::from(
        "my_username",
        "my_password",
        None,
        None,
        "https://my-directory-url.com",
    )
        .await;

    let (mqtt_client, receiver) = service_client
        .mqtt_interface
        .get_mqtt_client(MqttProtocol::TLS, 8883, "my_client_id")
        .await
        .expect("Couldn't create MQTT client");

    mqtt_client.subscribe("spBv1.0/my-group/+/my-node/my-device", 0);

    let mut message_iterator = receiver.iter();
    loop {
        if let Some((topic, payload)) = message_iterator.next() {
            println!("{}", payload);
        }
    }
}

```

## Limitations

There is no synchronous implementation of this service client.