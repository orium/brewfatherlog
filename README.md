[![Build Status](https://github.com/orium/brewfatherlog/workflows/CI/badge.svg)](https://github.com/orium/brewfatherlog/actions?query=workflow%3ACI)
[![Dependency status](https://deps.rs/repo/github/orium/brewfatherlog/status.svg)](https://deps.rs/repo/github/orium/brewfatherlog)
[![crates.io](https://img.shields.io/crates/v/brewfatherlog.svg)](https://crates.io/crates/brewfatherlog)
[![Downloads crates.io](https://img.shields.io/crates/d/brewfatherlog.svg?label=crates.io%20downloads)](https://crates.io/crates/brewfatherlog)
[![Downloads github](https://img.shields.io/github/downloads/orium/brewfatherlog/total.svg?label=github%20downloads)](https://github.com/orium/brewfatherlog/releases)
[![Github stars](https://img.shields.io/github/stars/orium/brewfatherlog.svg?logo=github)](https://github.com/orium/brewfatherlog/stargazers)
[![License](https://img.shields.io/crates/l/brewfatherlog.svg)](./LICENSE.md)


# Brewfatherlog

<!-- cargo-rdme start -->

Brewfatherlog is a small tool to synchronize the temperatures of your Grainfather fermenters to Brewfather.

## Instalation

Brewfatherlog can be installed via `cargo` with:

```bash
cargo install brewfatherlog
```

You can also get a binary from the [releases page](https://github.com/orium/brewfatherlog/releases/).

### Configuration

On the first run Brewfatherlog will create a configuration file in your configuration directory. Brewfatherlog will
tell you where the configuration file is. You will need to edit that file to configure authentication for
both Grainfather and Brewfather.

In Brewfather you need to enable the "Custom Stream" integration in the
[settings page](https://web.brewfather.app/tabs/settings) and put the logging id in the configuration file.

### Systemd daemon

WIP!

<!-- cargo-rdme end -->
