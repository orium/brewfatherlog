/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]

//! TODO

mod config;

use crate::config::Config;
use brewfatherlog::brewfather::{Brewfather, BrewfatherLoggingEvent};
use brewfatherlog::grainfather::Grainfather;

#[tokio::main]
async fn main() {
    Config::create_file_if_nonexistent();

    let config: Config = Config::from_config_file();

    if false {
        let brewfather = Brewfather::new(config.brewfather.logging_id);

        let event = BrewfatherLoggingEvent { name: "Ferm 1".to_owned(), temp: 21.1 };

        let r = brewfather.log(event).await;

        println!("result: {:?}", r);
    }

    let grainfather =
        Grainfather::new(&config.grainfather.auth.email, &config.grainfather.auth.password)
            .await
            .unwrap();

    let ferms = grainfather.list_fermenters().await.unwrap();

    for ferm in ferms {
        let temp = grainfather.get_fermenter_temperature(ferm.id).await.unwrap().unwrap();
        println!("{} {} C", ferm.name, temp.temperature);
    }
}
