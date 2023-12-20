/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]

//! Brewfatherlog is a small tool to synchronize the temperatures of your Grainfather fermenters to Brewfather.
//!
//! # Instalation
//!
//! Brewfatherlog can be installed via `cargo` with:
//!
//! ```bash
//! cargo install brewfatherlog
//! ```
//!
//! You can also get a binary from the [releases page](https://github.com/orium/brewfatherlog/releases/).
//!
//! ## Configuration
//!
//! On the first run Brewfatherlog will create a configuration file in your configuration directory. Brewfatherlog will
//! tell you where the configuration file is. You will need to edit that file to configure authentication for
//! both Grainfather and Brewfather.
//!
//! In Brewfather you need to enable the "Custom Stream" integration in the
//! [settings page](https://web.brewfather.app/tabs/settings) and put the logging id in the configuration file.
//!
//! ## Systemd daemon
//!
//! WIP!

mod config;

use crate::config::Config;
use brewfatherlog::brewfather::{Brewfather, BrewfatherLoggingEvent};
use brewfatherlog::grainfather::Grainfather;
use log::{error, info, warn};
use simplelog::{
    format_description, ColorChoice, CombinedLogger, LevelFilter, TermLogger, TerminalMode,
    WriteLogger,
};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

fn program_dir_path() -> PathBuf {
    let home_dir: PathBuf = dirs::home_dir().expect("Unable to get home directory");

    home_dir.join(format!(".{PROGRAM_NAME}"))
}

fn config_file_path() -> PathBuf {
    program_dir_path().join(format!("{PROGRAM_NAME}.toml"))
}

fn log_file_path() -> PathBuf {
    program_dir_path().join(format!("{PROGRAM_NAME}.log"))
}

fn init_logging() {
    let config = simplelog::ConfigBuilder::new()
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
        ))
        .build();

    let log_file_path = log_file_path();

    std::fs::create_dir_all(log_file_path.parent().unwrap())
        .expect("failed to create the program directory");

    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(log_file_path)
        .expect("failed to open log file");

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Info, config, log_file),
    ])
    .expect("failed to initialize loggers");
}

async fn main_loop(config: Config) {
    info!("Starting {}.", PROGRAM_NAME);

    let brewfather = Brewfather::new(config.brewfather.logging_id);

    loop {
        let grainfather =
            Grainfather::new(&config.grainfather.auth.email, &config.grainfather.auth.password)
                .await
                .unwrap();

        let ferms = match grainfather.list_fermenters().await {
            Ok(ferms) => ferms,
            Err(err) => {
                error!("Error getting fermenters: {:?}", err);
                continue;
            }
        };

        for ferm in ferms {
            let temp = match grainfather.get_fermenter_temperature(ferm.id).await {
                Ok(Some(temp)) => temp,
                Ok(None) => {
                    warn!("No recent temperature record of fermenter \"{}\".", ferm.name);
                    continue;
                }
                Err(err) => {
                    error!("Error getting temperature of fermenter \"{}\": {:?}", ferm.name, err);
                    continue;
                }
            };

            info!("Fermenter \"{}\": {:.02} C", ferm.name, temp.temperature);

            let event = BrewfatherLoggingEvent { name: &ferm.name, temp: temp.temperature };

            match brewfather.log(event).await {
                Ok(()) => {
                    info!("Logged temperature of fermenter \"{}\" to brewfather.", ferm.name);
                }
                Err(err) => {
                    error!(
                        "Error logging the temperature of fermenter \"{}\" to brewfather: {:?}",
                        ferm.name, err
                    );
                }
            }
        }

        sleep(Duration::from_secs(15 * 60 + 1)).await;
    }
}

// WIP! do not log twice
// WIP! do not log old

#[tokio::main]
async fn main() {
    let config_file = config_file_path();
    let created_config_file = Config::create_file_if_nonexistent(&config_file);

    if created_config_file {
        println!(
            "Created configuration file on \"{}\". Please edit it and run {} again.",
            config_file.display(),
            PROGRAM_NAME,
        );
        std::process::exit(0);
    }

    let config: Config = Config::from_config_file(&config_file);

    init_logging();

    main_loop(config).await;
}
