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
//! On the first run Brewfatherlog will create a configuration file in your configuration directory (in POSIX systems
//! that should be in `~/.config/`). You will need to edit that file to configure authentication for both Grainfather
//! and Brewfather.
//!
//! WIP! talk about enabling streaming logging in brewfather.
//!
//! ## Systemd daemon
//!
//! WIP!

mod config;

use crate::config::Config;
use brewfatherlog::brewfather::{Brewfather, BrewfatherLoggingEvent};
use brewfatherlog::grainfather::Grainfather;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    Config::create_file_if_nonexistent();
    // WIP! if the file did not exist we should print a nice message telling the user to edit the file and exit.

    let config: Config = Config::from_config_file();

    let brewfather = Brewfather::new(config.brewfather.logging_id);

    loop {
        let grainfather =
            Grainfather::new(&config.grainfather.auth.email, &config.grainfather.auth.password)
                .await
                .unwrap();

        let ferms = grainfather.list_fermenters().await.unwrap();

        for ferm in ferms {
            let temp = grainfather.get_fermenter_temperature(ferm.id).await.unwrap().unwrap();

            println!("{} {} C", ferm.name, temp.temperature);

            let event = BrewfatherLoggingEvent { name: ferm.name, temp: temp.temperature };

            let r = brewfather.log(event).await;
            println!("  log result: {:?}", r);
        }

        sleep(Duration::from_secs(15 * 60 + 1)).await;
    }
}
