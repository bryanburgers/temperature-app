//! A small program that hits the GraphQL endpoint and inserts test data. This is a convenience so
//! we have data in the system. I guess this more or less simulates what the BLE data collector
//! would be doing.

use clap::{App, Arg};
use reqwest;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

/// The mutation query that we'll use to insert the data.
const QUERY: &'static str = r#"
    mutation ($address: String!, $temp: Celsius!) {
        addMeasurement(address:$address, tempC: $temp) {
            date
            tempC
            tempRawC
        }
    }
"#;

/// Start a thread that inserts dummy data into the system.
///
/// This essentially inserts sine-wave-shaped data into the system. `min` is the trough of the sine
/// wave. `max` is the crest of the sine wave. And `period` is how long (in real-world time) the
/// sine wave lasts.
fn spawn_dummy(
    client: Arc<reqwest::Client>,
    url: Url,
    address: String,
    min: f64,
    max: f64,
    period: Duration,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let start = Instant::now();
        let sine_scale = 2.0 * std::f64::consts::PI / period.as_millis() as f64;

        loop {
            let elapsed = start.elapsed().as_millis() as f64;
            let sine = ((elapsed * sine_scale).sin() + 1.0) / 2.0;
            let value = sine * (max - min) + min;

            let result = client
                .post(url.as_str())
                .json(&json!({
                    "query": QUERY,
                    "variables": {
                        "address": address,
                        "temp": value,
                    },
                }))
                .send();
            match result {
                Ok(_) => println!("{}: {}", address, value),
                Err(e) => println!("{}", e),
            };
            thread::sleep(Duration::from_secs(2));
        }
    })
}

fn main() {
    // Set up command-line arguments
    let matches = App::new("dummy-data-loader")
        .version("0.1.0")
        .author("Bryan Burgers <bryan@burgers.io>")
        .about("Load dummy data into the graphql server")
        .arg(
            Arg::with_name("endpoint")
                .short("e")
                .long("endpoint")
                .value_name("URL")
                .help("The URL of the GraphQL server")
                .takes_value(true)
                .validator(|s| match Url::parse(&s) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Invalid url: {}", e)),
                })
                .default_value("http://127.0.0.1:8080/graphql"),
        )
        .get_matches();

    // Get the database address for the ElasticSearch server to connect to from the command line.
    // We know all of these unwraps are valid because we had clap validate them for us already.
    let url = Url::parse(matches.value_of("endpoint").unwrap()).unwrap();

    let client = Arc::new(reqwest::Client::new());

    let child1 = spawn_dummy(
        client.clone(),
        url.clone(),
        "f4d55889b1d6".into(),
        16.667,                  // trough of sine wave
        20.0,                    // crest of sine wave
        Duration::from_secs(70), // period of sine wave
    );

    thread::sleep(Duration::from_secs(1));

    let child2 = spawn_dummy(
        client.clone(),
        url.clone(),
        "d0f7083ca3b1".into(),
        24.88,                    // trough of sine wave
        30.0,                     // crest of sine wave
        Duration::from_secs(120), // period of sine wave
    );

    child1.join().unwrap();
    child2.join().unwrap();
}
