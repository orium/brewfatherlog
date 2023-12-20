/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, COOKIE, ORIGIN};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroU8;
use time::format_description::well_known::iso8601::TimePrecision;
use time::format_description::well_known::{iso8601, Iso8601};
use time::{Duration, OffsetDateTime};

const CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(thiserror::Error, Debug)]
pub enum GrainfatherError {
    #[error("transport error: {error}")]
    Transport {
        #[from]
        error: reqwest::Error,
    },
    #[error("no set cookie header")]
    NoSetCookieHeader,
    #[error("invalid set cookie header")]
    InvalidSetCookieHeader,
    #[error("unexpected status code {status_code}: \"{payload}\"")]
    UnexpectedStatusCode { status_code: StatusCode, payload: String },
    #[error("error parsing response: \"{payload}\"")]
    ResponseParsing { payload: String },
    #[error("invalid timestamp: \"{error}\"")]
    ResponseTimestamp { error: time::error::ComponentRange },
    #[error("unable to find CSRF token")]
    UnableToFindCSRFToken,
    #[error("unable to find auth cookie")]
    UnableToFindAuthCookie,
}

#[derive(Deserialize, Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct FermenterId(u64);

impl FermenterId {
    fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Deserialize, Debug)]
pub struct Fermenter {
    pub id: FermenterId,
    pub name: String,
}

#[derive(Debug)]
pub struct TemperatureRecord {
    pub temperature: f32,
    pub timestamp: OffsetDateTime,
}

pub struct Grainfather {
    auth_cookie: String,
    client: reqwest::Client,
}

impl Grainfather {
    pub async fn new(email: &str, password: &str) -> Result<Grainfather, GrainfatherError> {
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(CONNECTION_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        let auth_cookie = Grainfather::login(&client, email, password).await?;

        Ok(Grainfather { auth_cookie, client })
    }

    async fn login(
        client: &reqwest::Client,
        email: &str,
        password: &str,
    ) -> Result<String, GrainfatherError> {
        #[derive(Serialize)]
        struct Request<'a> {
            email: &'a str,
            password: &'a str,
            remember: bool,
        }

        let request = Request { email, password, remember: false };

        let http_response = client.get("https://community.grainfather.com/login").send().await?;

        let cookies: HeaderMap = match http_response.status() {
            StatusCode::OK => http_response
                .cookies()
                .map(|c| {
                    (
                        COOKIE,
                        HeaderValue::from_str(&format!("{}={}", c.name(), c.value()))
                            .expect("invalid header value"),
                    )
                })
                .collect(),
            status_code => {
                let payload = http_response.text().await?;
                return Err(GrainfatherError::UnexpectedStatusCode { status_code, payload });
            }
        };

        // Ugly hack to get the token. This might break in the future.
        let csrf_token = {
            let response_text = http_response.text().await?;

            response_text
                .split(',')
                .find(|t| t.contains("csrfToken"))
                .and_then(|t| t.split_once(':'))
                .map(|(_, token)| token.replace('"', ""))
                .ok_or(GrainfatherError::UnableToFindCSRFToken)?
        };

        let http_response = client
            .post("https://community.grainfather.com/login")
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json, text/plain, */*")
            .header("X-CSRF-TOKEN", csrf_token)
            .headers(cookies)
            .json(&request)
            .send()
            .await?;

        match http_response.status() {
            StatusCode::OK => http_response
                .cookies()
                .find(|c| c.name().starts_with("remember_web_"))
                .map(|c| format!("{}={}", c.name(), c.value()))
                .ok_or(GrainfatherError::UnableToFindAuthCookie),
            status_code => {
                let payload = http_response.text().await?;
                Err(GrainfatherError::UnexpectedStatusCode { status_code, payload })
            }
        }
    }

    pub async fn list_fermenters(&self) -> Result<Vec<Fermenter>, GrainfatherError> {
        let http_response = self
            .client
            .get("https://community.grainfather.com/my-equipment/fermentation-device/data")
            .header(ACCEPT, "application/json")
            .header(COOKIE, &self.auth_cookie)
            .send()
            .await?;

        match http_response.status() {
            StatusCode::OK => Ok(http_response.json::<Vec<Fermenter>>().await?),
            status_code => {
                let payload = http_response.text().await?;
                Err(GrainfatherError::UnexpectedStatusCode { status_code, payload })
            }
        }
    }

    pub async fn get_fermenter_temperature(
        &self,
        fermenter_id: FermenterId,
    ) -> Result<Option<TemperatureRecord>, GrainfatherError> {
        #[derive(Deserialize, Debug)]
        struct Response {
            temperature: Vec<(i64, f32)>,
        }

        const DATETIME_FORMAT: iso8601::EncodedConfig = iso8601::Config::DEFAULT
            .set_time_precision(TimePrecision::Second { decimal_digits: NonZeroU8::new(3) })
            .encode();

        let from = OffsetDateTime::now_utc() - Duration::HOUR;
        let url = format!(
            "https://community.grainfather.com/my-equipment/fermentation-device/{}/history?from={}",
            fermenter_id.as_u64(),
            from.format(&Iso8601::<DATETIME_FORMAT>).expect("failed to format time"),
        );

        let http_response = self
            .client
            .get(url)
            .header(ACCEPT, "application/json")
            .header(COOKIE, &self.auth_cookie)
            .header(ORIGIN, "https://community.grainfather.com")
            .send()
            .await?;

        let status_code = http_response.status();
        let payload = http_response.text().await?;

        match status_code {
            StatusCode::OK => serde_json::from_str::<HashMap<String, serde_json::Value>>(&payload)
                .map_err(|_| GrainfatherError::ResponseParsing { payload })?
                .into_values()
                .find(|v| v.as_object().is_some_and(|m| m.contains_key("temperature")))
                .and_then(|v| serde_json::from_value::<Response>(v).ok())
                .and_then(|r| r.temperature.into_iter().max_by_key(|(timestamp, _)| *timestamp))
                .map(|(timestamp, temperature)| {
                    Ok(TemperatureRecord {
                        temperature,
                        timestamp: OffsetDateTime::from_unix_timestamp(timestamp / 1000)
                            .map_err(|error| GrainfatherError::ResponseTimestamp { error })?,
                    })
                })
                .transpose(),
            status_code => Err(GrainfatherError::UnexpectedStatusCode { status_code, payload }),
        }
    }
}
