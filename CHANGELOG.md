# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased (0.5.0)

### Breaking changes

* Update to **tonic** 0.8
* Update to **prost** 0.11

## 0.4.0 (2022-05-11)

* Add `ServiceDefinition::try_from((String, u16))` implementation.

### Breaking changes

* Update to **tonic** 0.7
* Update to **prost** 0.10

## 0.3.0 (2021-10-25)

### Breaking changes

* `LoadBalancedChannelBuilder::channel` is now async and fallible.
* `LoadBalancedChannel::builder` is now sync.
* `ServiceDefinition::hostname` and `ServiceDefinition::port` are now made private.
* Update to **tonic** 0.6
* Update to **prost** 0.9

### Features

* Add `ResolutionStrategy` to be able to resolve IPs once on startup [#2](https://github.com/TrueLayer/ginepro/issues/20) [d72678d](https://github.com/TrueLayer/ginepro/commit/d72678dc10342a83ecd0e66d10d9ac46469ba91b).
* Add `TryFrom` constructor for `ServiceDefinition` that verifies `hostname`.
