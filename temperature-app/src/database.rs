//! Access to the ElasticSearch database
//!
//! Provide access to an ElasticSearch database and perform key operations against the database.
//!
//! ```
//! # use temperature_app::database::Database;
//! let url = url::Url::parse("http://localhost:9200").unwrap();
//! let database = Database::new(url);
//! let ble_address = "f4d55889b1d6";
//! let now = chrono::Utc::now();
//! let temperature = 27.0.into();
//! database.insert_measurement(ble_address, now, temperature);
//! ```

use crate::temperature::Celsius;
use chrono::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

/// A connection to the ElasticSearch database
pub struct Database {
    url: Url,
    client: reqwest::Client,
}

/// Errors that can occur when using the database.
pub enum DatabaseError {
    /// An attempt to build a URL failed.
    InvalidUrl,
    /// Requests to the specified endpoint failed to connect.
    RequestFailed,
    /// The response from the specified endpoint was not valid JSON.
    InvalidJson,
    /// The json returned from the specified endpoint did not match what we expected it to look
    /// like.
    UnexpectedResponse,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            DatabaseError::InvalidUrl => "Invalid URL".fmt(f),
            DatabaseError::RequestFailed => "The request to the database failed".fmt(f),
            DatabaseError::InvalidJson => {
                "The request to the database returned invalid JSON".fmt(f)
            }
            DatabaseError::UnexpectedResponse => {
                "The requested to the database returned unexpected results".fmt(f)
            }
        }
    }
}

/// The result of a request for measurements from the database
pub struct MeasurementResult {
    /// The address for this measurement
    pub address: Option<String>,
    /// The data that the measurement was taken
    pub date: Option<DateTime<Utc>>,
    /// The raw temperature reading at the given time
    pub temperature: Option<Celsius>,
}

/// Used internally for deserializing from ElasticSearch.
#[derive(Debug, Serialize, Deserialize)]
struct Hit {
    _id: String,
    _index: String,
    _score: Option<f64>,
    _source: HitSource,
    _type: String,
}

/// Used internally for deserializing from ElasticSearch.
#[derive(Debug, Serialize, Deserialize)]
struct HitSource {
    address: Option<String>,
    temp_c: Option<f64>,
    date: Option<DateTime<Utc>>,
}

impl Database {
    /// Create a new database connection to the ElasticSearch database found at the specified URL.
    pub fn new(url: Url) -> Self {
        let client = reqwest::Client::new();

        Database { url, client }
    }

    /// Insert a measurement into the database.
    ///
    /// Note that we store data in one-second resolution, so inserting multiple times per second
    /// will result in updated values instead of new, distinct values.
    pub fn insert_measurement(
        &self,
        address: &str,
        date: DateTime<Utc>,
        temperature: Celsius,
    ) -> Result<(), DatabaseError> {
        // Drop sub-second precision. We're only storing second resolution. It's safe to unwrap
        // because dropping the nanosecond precision won't make this an invalid date.
        let date = date.with_nanosecond(0).unwrap();
        // Use the current day as the index. This way, we can drop days worth of old data.
        let index = format!("{}", date.format("%Y%m%d"));
        // Create an ID out of the address and the date. If we get another measurement for this
        // same exact second, we will overwrite rather than add.
        let id = format!("{}-{}", date.format("%Y%m%dT%H%M%S"), address);
        // Join those to make a full path.
        let path = format!("{}/_doc/{}", index, id);
        // Build the PUT url.
        let url = match self.url.join(&path) {
            Ok(url) => url,
            Err(_) => return Err(DatabaseError::InvalidUrl),
        };

        // Put the data into elasticsearch.
        let result = self
            .client
            .put(url.as_str())
            .json(&json!({
                "address": address,
                "date": date.to_rfc3339(),
                "temp_c": f64::from(temperature),
            }))
            .send();

        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::RequestFailed),
        }
    }

    /// Get measurements for the specified device
    ///
    /// TODO: If this went further, there would have to be more control here like order, limit,
    /// since, pagniation, etc. so that we can allow the user to really get which measurements they
    /// want. But for this project, we'll stop at providing a limit and always show the most recent
    /// values.
    pub fn select_measurements_for_device(
        &self,
        address: &str,
        limit: u32,
    ) -> Result<Vec<MeasurementResult>, DatabaseError> {
        let url = match self.url.join("/*/_search") {
            Ok(url) => url,
            Err(_) => return Err(DatabaseError::InvalidUrl),
        };

        let result = self
            .client
            .post(url.as_str())
            .json(&json!({
                "size": limit,
                "sort": {
                    "date": "desc",
                },
                "query": {
                    "bool" : {
                        "filter" : {
                            "term" : { "address" : address },
                        }
                    }
                }
            }))
            .send();

        let mut response = match result {
            Ok(response) => response,
            Err(_) => return Err(DatabaseError::RequestFailed),
        };

        let value: serde_json::Value = match response.json() {
            Ok(value) => value,
            Err(_) => return Err(DatabaseError::InvalidJson),
        };

        // ElasticSearch returns the data as a hits top-level key, which is an object that contains
        // another hits key, which is then the array of hits.
        let hits: &serde_json::Value = match value.get("hits") {
            Some(hits) => hits,
            None => return Err(DatabaseError::UnexpectedResponse),
        };
        let hits: &serde_json::Value = match hits.get("hits") {
            Some(hits) => hits,
            None => return Err(DatabaseError::UnexpectedResponse),
        };
        let hits: serde_json::Value = hits.clone();

        // Now use serde to transform all of the actual hits into our internal Hit type.
        let items: Vec<Hit> = match serde_json::value::from_value(hits) {
            Ok(hits) => hits,
            Err(_) => return Err(DatabaseError::UnexpectedResponse),
        };

        let mut measurements: Vec<MeasurementResult> = items
            .into_iter()
            .map(|hit| MeasurementResult {
                address: hit._source.address,
                date: hit._source.date,
                temperature: hit._source.temp_c.map(|val| val.into()),
            })
            .collect();

        measurements.reverse();

        Ok(measurements)
    }
}
