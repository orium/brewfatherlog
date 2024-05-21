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
//! To make Brewfatherlog a systemd service that will start automatically create file
//! `/etc/systemd/system/brewfatherlog.service` with the content (replace the user and the path to the brewfatherlog
//! binary):
//!
//! ```ini
//! [Unit]
//! Description=Log temperatures from grainfather fermenters to brewfather
//! After=network.target
//!
//! [Service]
//! Type=simple
//! Restart=always
//! RestartSec=1
//! User=<USER>
//! ExecStart=<PATH TO brewfatherlog>
//!
//! [Install]
//! WantedBy=multi-user.target
//! ```
//!
//! and then enable and start the service:
//!
//! ```bash
//! systemctl enable brewfatherlog
//! systemctl start brewfatherlog
//! ```

mod config;

use crate::config::Config;
use brewfatherlog::brewfather::{Brewfather, BrewfatherLoggingEvent};
use brewfatherlog::grainfather::{Fermenter, FermenterId, Grainfather, TemperatureRecord};
use log::{error, info, warn};
use simplelog::{
    format_description, ColorChoice, CombinedLogger, LevelFilter, TermLogger, TerminalMode,
    WriteLogger,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;

pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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

async fn log_temperature(
    brewfather: &Brewfather,
    last_logged: &mut HashMap<FermenterId, OffsetDateTime>,
    fermenter: &Fermenter,
    temp_record: TemperatureRecord,
) {
    let now = OffsetDateTime::now_utc();
    let age: time::Duration = now - temp_record.timestamp;

    if age > Duration::from_secs(30 * 60) {
        warn!(
            "Ignoring temperature {:.02} °C of fermenter \"{}\" because the temperature is too old ({}).",
            temp_record.temperature,
            fermenter.name,
            humantime::format_duration(Duration::from_secs(age.whole_seconds() as u64)),
        );
        return;
    }

    if last_logged.get(&fermenter.id).is_some_and(|last| now <= *last) {
        warn!(
            "Ignoring temperature {:.02} °C of fermenter \"{}\" because we already logged it.",
            temp_record.temperature, fermenter.name,
        );
        return;
    }

    let event = BrewfatherLoggingEvent { name: &fermenter.name, temp: temp_record.temperature };

    match brewfather.log(event).await {
        Ok(()) => {
            info!("Logged temperature of fermenter \"{}\" to brewfather.", fermenter.name);
            last_logged.insert(fermenter.id, temp_record.timestamp);
        }
        Err(err) => {
            error!(
                "Error logging the temperature of fermenter \"{}\" to brewfather: {}",
                fermenter.name, err
            );
        }
    }
}

async fn main_loop(config: Config) -> ! {
    let init_grainfather =
        || Grainfather::new(&config.grainfather.auth.email, &config.grainfather.auth.password);

    info!("Starting {} v{}.", PROGRAM_NAME, VERSION);

    let mut last_logged: HashMap<FermenterId, OffsetDateTime> = HashMap::new();

    let brewfather = Brewfather::new(config.brewfather.logging_id)
        .expect("error initializing brewfather client");

    loop {
        let grainfather = match init_grainfather().await {
            Ok(grainfather) => grainfather,
            Err(err) => {
                error!("Error initializing grainfather client: {}", err);
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        let ferms = match grainfather.list_fermenters().await {
            Ok(ferms) => ferms,
            Err(err) => {
                error!("Error getting fermenters: {}", err);
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        if ferms.is_empty() {
            info!("No fermenters found.");
        }

        for ferm in ferms {
            match grainfather.get_fermenter_temperature(ferm.id).await {
                Ok(Some(temp_record)) => {
                    info!("Fermenter \"{}\": {:.02} °C", ferm.name, temp_record.temperature);

                    log_temperature(&brewfather, &mut last_logged, &ferm, temp_record).await;
                }
                Ok(None) => {
                    info!("No recent temperature record of fermenter \"{}\".", ferm.name);
                }
                Err(err) => {
                    error!("Error getting temperature of fermenter \"{}\": {}", ferm.name, err);
                }
            }
        }

        sleep(Duration::from_secs(15 * 60 + 1)).await;
    }
}

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
