# ginepro

`ginepro` provides client-side gRPC load-balancing out of the box by enriching `tonic` ‘s channel with periodic service discovery.

[![Crates.io](https://img.shields.io/crates/v/ginepro.svg)](https://crates.io/crates/ginepro)
[![Docs.rs](https://docs.rs/ginepro/badge.svg)](https://docs.rs/ginepro)
[![CI](https://github.com/TrueLayer/ginepro/workflows/CI/badge.svg)](https://github.com/TrueLayer/ginepro/actions)
[![Coverage Status](https://coveralls.io/repos/github/TrueLayer/ginepro/badge.svg?branch=main&t=UWgSpm)](https://coveralls.io/github/TrueLayer/ginepro?branch=main)

# Overview

`ginepro` enriches [tonic](https://github.com/hyperium/tonic) by periodcally updating the list of
servers that are available through a `ServiceDiscovery` interface that currently is implemented for DNS.

## How to install

Add `ginepro` to your dependencies

```toml
[dependencies]
# ...
ginepro = "0.2.0"
```

## Getting started

The interface remains fairly the same as we implement all the logic for a drop-in replacement for
tonic's `Channel`.

```rust
// Using the `LoadBalancedChannel`.
use ginepro::LoadBalancedChannel;
use ginepro::pb::tester_client::TesterClient;

// Build a load-balanced channel given a service name and a port.
let load_balanced_channel = LoadBalancedChannel::builder(
    ("my_hostname", 5000)
  )
  .channel()
  .await.expect("failed to construct LoadBalancedChannel");

// Initialise a new gRPC client for the `Test` service
// using the load-balanced channel as transport
let grpc_client = TesterClient::new(load_balanced_channel);
```

For more examples, have a look at the [examples](ginepro/examples) directory.

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>
