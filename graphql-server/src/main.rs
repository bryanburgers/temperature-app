//! The binary that implements the graphql server. I hadn't used any of the suggested frameworks,
//! so I arbitrarily went with Warp.
//!
//! This binary pulls in most of its logic from the `temperature_app` library, and just does what
//! it takes to start and configure the server.

use clap::{App, Arg};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use temperature_app::{
    database::Database,
    graphql::{schema, Context, Device},
};
use url::Url;
use warp::{http::Response, Filter};

fn main() {
    // Set up command-line arguments
    let matches = App::new("graphql-server")
        .version("0.1.0")
        .author("Bryan Burgers <bryan@burgers.io>")
        .about("GraphQL server for temperature measurements")
        .arg(
            Arg::with_name("listen")
                .short("l")
                .long("listen")
                .value_name("SOCKADDR")
                .help("Which socket address to listen on")
                .takes_value(true)
                .validator(|s| match s.parse::<std::net::SocketAddr>() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Invalid socket address: {}", e)),
                })
                .default_value("127.0.0.1:8080"),
        )
        .arg(
            Arg::with_name("database")
                .short("d")
                .long("database")
                .value_name("URL")
                .help("The URL of the ElasticSearch database")
                .takes_value(true)
                .validator(|s| match Url::parse(&s) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Invalid url: {}", e)),
                })
                .default_value("http://127.0.0.1:9200"),
        )
        .arg(
            Arg::with_name("sensors")
                .short("s")
                .long("sensors")
                .value_name("FILE")
                .help("The location of the toml file that contains sensor information")
                .takes_value(true),
        )
        .get_matches();

    // Get the listen address for the server to listen on from the command line. We know all of
    // these unwraps are valid because we had clap validate them for us already.
    let socket_address: std::net::SocketAddr = matches.value_of("listen").unwrap().parse().unwrap();

    // Get the database address for the ElasticSearch server to connect to from the command line.
    // We know all of these unwraps are valid because we had clap validate them for us already.
    let database_url = Url::parse(matches.value_of("database").unwrap()).unwrap();

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(include_str!("index.html"))
    });

    println!("Listening on {}", socket_address);

    // Create the context. First, the database.
    let database = Arc::new(Database::new(database_url));
    // Then the list of known devices.
    let mut devices = BTreeMap::new();

    // If requsted and possible, load known devices from a sensors.toml config file. If this
    // project went further, we'd probably want to put these in a database somewhere, too, and have
    // GraphQL mutations to give sensors known names. But for now, a config file is fine.
    if let Some(sensors_path) = matches.value_of("sensors") {
        match load_sensors(sensors_path) {
            Ok(config) => {
                for sensor in config.sensors {
                    let device = Device {
                        address: sensor.address.clone(),
                        name: sensor.name,
                        description: sensor.description,
                        adjustment: sensor.adjustment.unwrap_or(0.0).into(),
                    };
                    devices.insert(sensor.address.into(), device);
                }
            }
            Err(e) => {
                eprintln!("Could not load sensor configuration: {}", e);
            }
        }
    }
    let devices = Arc::new(devices);

    // Create the warp state with our database/devices context.
    let state = warp::any().map(move || Context {
        devices: devices.clone(),
        database: database.clone(),
    });
    let graphql_filter = juniper_warp::make_graphql_filter(schema(), state.boxed());

    // Here we go!
    warp::serve(
        warp::get2()
            .and(warp::path("graphiql"))
            .and(juniper_warp::graphiql_filter("/graphql"))
            .or(homepage)
            .or(warp::path("graphql").and(graphql_filter)),
    )
    .run(socket_address);
}

/// The structure that represents the sensors.toml file
#[derive(Debug, Deserialize)]
struct ConfigFile {
    sensors: Vec<ConfigSensor>,
}

/// A single sensor in the sensors.toml file
#[derive(Debug, Deserialize)]
struct ConfigSensor {
    address: String,
    name: Option<String>,
    description: Option<String>,
    adjustment: Option<f64>,
}

/// Load a sensors.toml file
fn load_sensors(path: impl AsRef<Path>) -> Result<ConfigFile, std::io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: ConfigFile = toml::from_str(&contents).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to read config file: {}", e),
        )
    })?;

    Ok(config)
}
