//! A project that exposes temperature measurements over GraphQL.
//!
//! ## Background
//!
//! I've been playing with a couple of new Arduinos at my house in the hopes of capturing the
//! current temperature at various points in my house. These devices are capable of making
//! BlueTooth Low Energy (BLE) advertisements, and I plan to have a Raspberry Pi listening for
//! these advertisements and tracking the temperature.
//!
//! It's really more for fun than to accomplish anything specific, but when I'm done I hope to be
//! able to see the current temperatures, the spread between the temperatures, and a historic
//! graph.
//!
//! ## This project
//!
//! What I'm doing for that project seems to fit right into a nice little project. This project
//! uses ElasticSearch and GraphQL to create a data store for the temperature measurements.
//!
//! (I opted to focus on ElasticSearch and GraphQL for this project because I did the bulk of it
//! while in airports and on airplanes, so I wanted something that could be fully contained on my
//! laptop. Lambda and DynamoDB didn't fit that bill.)
//!
//! The idea is that there is a mutation called `addMeasurement` that adds a single temperature
//! measurement to ElasticSearch.
//!
//! ```graphql
//! mutation {
//!   addMeasurement(address: "f4d55889b1d6", tempC: 29.0) {
//!     date
//!   }
//! }
//! ```
//!
//! And there is a query called `device` which returns information about a device and can return
//! its measurements.
//!
//! ```graphql
//! query {
//!   device(address:"f4d55889b1d6") {
//!     address
//!     name
//!     description
//!     currentMeasurement {
//!       date
//!       tempF
//!     }
//!     measurements(count: 2) {
//!       date
//!       tempF
//!       tempC
//!       tempRawC
//!     }
//!   }
//! }
//! ```
//!
//! *Most* of the logic is contained inside this library so that `cargo doc` can be used to
//! generate documentation. Two binaries also exist:
//!
//! 1. `graphql-server` serves the GraphQL endpoint on an HTTP server.
//! 2. `dummy-data-loader` sends adds dummy measurements to the system via the GraphQL endpoint.
//!
//! ## Usage
//!
//! Docker Compose is all set up so that the following script will get everything up-and-running.
//!
//! ```bash
//! docker-compose build
//! docker-compose up
//! ```
//!
//! Then navigate to [http://localhost:8080](http://localhost:8080) to see some data. (Note that it
//! takes the ElasticSearch container a little while to boot up, so there may not be data right
//! away.) ElasticSearch is exposed on port 9200.

#![deny(missing_docs)]
pub mod database;
pub mod graphql;
pub mod temperature;
