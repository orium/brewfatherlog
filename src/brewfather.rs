/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(thiserror::Error, Debug)]
pub enum BrewfatherError {
    #[error("transport error: {error}")]
    Transport {
        #[from]
        error: reqwest::Error,
    },
    #[error("unexpected status code {status_code}: \"{payload}\"")]
    UnexpectedStatusCode { status_code: StatusCode, payload: String },
    #[error("unexpected result: \"{result}\"")]
    UnexpectedResult { result: String },
    #[error("no result")]
    NoResult,
}

#[derive(Serialize)]
pub struct BrewfatherLoggingEvent<'a> {
    pub name: &'a str,
    pub temp: f32,
}

pub struct Brewfather {
    logging_id: String,
    client: reqwest::Client,
}

impl Brewfather {
    pub fn new(logging_id: impl Into<String>) -> Result<Brewfather, BrewfatherError> {
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(CONNECTION_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Brewfather { logging_id: logging_id.into(), client })
    }

    pub async fn log(&self, event: BrewfatherLoggingEvent<'_>) -> Result<(), BrewfatherError> {
        #[derive(Deserialize)]
        struct Response {
            result: Option<String>,
        }

        let url = format!("https://log.brewfather.net/stream?id={}", self.logging_id);

        let http_response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .json(&event)
            .send()
            .await?;

        match http_response.status() {
            StatusCode::OK => {
                let response = http_response.json::<Response>().await?;

                match response.result.as_deref() {
                    Some("OK" | "success") => Ok(()),
                    Some(result) => {
                        Err(BrewfatherError::UnexpectedResult { result: result.to_owned() })
                    }
                    None => Err(BrewfatherError::NoResult),
                }
            }
            status_code => {
                let payload = http_response.text().await?;
                Err(BrewfatherError::UnexpectedStatusCode { status_code, payload })
            }
        }
    }
}
