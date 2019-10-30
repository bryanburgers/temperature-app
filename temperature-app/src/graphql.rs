//! All the bits and bobs that deal with being a GraphQL server

use crate::{
    database::Database,
    temperature::{Celsius, Fahrenheit},
};
use chrono::prelude::*;
use chrono::{DateTime, Utc};
use juniper::FieldResult;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A known device
#[derive(Clone)]
pub struct Device {
    /// The BLE address of the device
    pub address: String,
    /// The human-readable name of the device, if available.
    pub name: Option<String>,
    /// The human-readable description for the device, if available.
    pub description: Option<String>,
    /// How to adjust the raw readings, in case of a miscalibrated temperature sensor, in degrees
    /// celsius.
    pub adjustment: Celsius,
}

/// A device according to our GraphQL layer. The device might be known or unknown.
#[derive(Clone)]
enum DeviceRef<'a> {
    /// A device that we know about because of our sensors.toml config file
    Known(&'a Device),
    /// A device that wasn't in our sensors.toml file, but may still have data associated with it.
    Unknown(String),
}

impl<'a> DeviceRef<'a> {
    /// How far to adjust the temperatures for this device, in degrees celsius.
    fn adjustment(&self) -> Celsius {
        match self {
            DeviceRef::Known(device) => device.adjustment,
            DeviceRef::Unknown(_) => 0.0.into(),
        }
    }
}

#[juniper::object(
    Context = Context,
)]
impl<'a> DeviceRef<'a> {
    /// The BLE address of the device.
    fn address(&self) -> String {
        match self {
            DeviceRef::Known(device) => device.address.clone(),
            DeviceRef::Unknown(address) => address.clone(),
        }
    }

    /// The human-readable name of the device, if available.
    fn name(&self) -> Option<String> {
        match self {
            DeviceRef::Known(device) => device.name.clone(),
            DeviceRef::Unknown(address) => None,
        }
    }

    /// A human-readable description of the device, if available.
    fn description(&self) -> Option<String> {
        match self {
            DeviceRef::Known(device) => device.description.clone(),
            DeviceRef::Unknown(address) => None,
        }
    }

    /// How far to adjust the temperatures for this device, in degrees celsius.
    fn adjustment(&self) -> Celsius {
        self.adjustment()
    }

    /// The current (most recent) measurement for this device.
    fn current_measurement(&self, context: &Context) -> FieldResult<Option<Measurement>> {
        let address: &str = match self {
            DeviceRef::Known(ref device) => &device.address,
            DeviceRef::Unknown(ref address) => address,
        };
        let measurements = context
            .database
            .select_measurements_for_device(address, 1)?;

        let measurement: Option<Measurement> = measurements
            .into_iter()
            .filter_map(
                |measurement| match (measurement.temperature, measurement.date) {
                    (Some(temperature), Some(date)) => Some(Measurement {
                        device: self.clone(),
                        date: date,
                        temperature: temperature.into(),
                    }),
                    _ => None,
                },
            )
            .nth(0);

        Ok(measurement)
    }

    /// Measurements for this device.
    fn measurements(&self, context: &Context, count: Option<i32>) -> FieldResult<Vec<Measurement>> {
        let address: &str = match self {
            DeviceRef::Known(ref device) => &device.address,
            DeviceRef::Unknown(ref address) => address,
        };
        let count = std::cmp::min(count.unwrap_or(10), 100) as u32;

        let measurements = context
            .database
            .select_measurements_for_device(address, count)?;

        let measurements: Vec<Measurement> = measurements
            .into_iter()
            .filter_map(
                |measurement| match (measurement.temperature, measurement.date) {
                    (Some(temperature), Some(date)) => Some(Measurement {
                        device: self.clone(),
                        date: date,
                        temperature: temperature.into(),
                    }),
                    _ => None,
                },
            )
            .collect();

        Ok(measurements)
    }
}

/// Data about a measurement.
struct Measurement<'a> {
    device: DeviceRef<'a>,
    date: DateTime<Utc>,
    temperature: Celsius,
}

impl<'a> Measurement<'a> {
    /// The adjusted value, based on the device that this measurement belong to.
    fn adjusted_temperature(&self) -> Celsius {
        let adjustment = self.device.adjustment();
        self.temperature + adjustment
    }
}

#[juniper::object()]
impl<'a> Measurement<'a> {
    /// The date and time that the measurement was taken.
    fn date(&self) -> DateTime<Utc> {
        self.date
    }

    /// The temperature, in degrees celsius
    fn temp_c(&self) -> Celsius {
        self.adjusted_temperature()
    }

    /// THe temperature, in degrees fahrenheit
    fn temp_f(&self) -> Fahrenheit {
        self.adjusted_temperature().into()
    }

    /// The raw (unadjusted) sensor temperature
    fn temp_raw_c(&self) -> f64 {
        self.temperature.into()
    }
}

/// Context that is passed to GraphQL queries
pub struct Context {
    /// The ElasticSearch database
    pub database: Arc<Database>,
    /// A list of devices
    pub devices: Arc<BTreeMap<String, Device>>,
}

// To make our context usable by Juniper, we have to implement a marker trait.
impl juniper::Context for Context {}

/// The GraphQL object that represents the base Query interface.
pub struct Query;

#[juniper::object(
    Context = Context,
)]
impl Query {
    pub fn device(context: &Context, address: String) -> FieldResult<DeviceRef> {
        let device: DeviceRef = match context.devices.get(&address) {
            Some(device) => DeviceRef::Known(device),
            None => DeviceRef::Unknown(address),
        };

        Ok(device)
    }
}

// Now, we do the same for our Mutation type.

/// The GraphQL object that represents the base Mutation interface.
pub struct Mutation;

#[juniper::object(
    Context = Context,
)]
impl Mutation {
    pub fn addMeasurement(
        context: &Context,
        address: String,
        temp_c: Celsius,
        date: Option<DateTime<Utc>>,
    ) -> FieldResult<Measurement> {
        let date = date.unwrap_or(Utc::now()).with_nanosecond(0).unwrap();

        context
            .database
            .insert_measurement(&address, date, temp_c)?;

        let device: DeviceRef = match context.devices.get(&address) {
            Some(ref device) => DeviceRef::Known(device),
            None => DeviceRef::Unknown(address),
        };

        Ok(Measurement {
            device: device,
            date: date,
            temperature: temp_c,
        })
    }
}

/// The type that represents the root of our GraphQL schema.
pub type Schema = juniper::RootNode<'static, Query, Mutation>;

/// Create a new schema.
///
/// I'm not actually very familiar with this. It was given in a Juniper example, and I kept it.
pub fn schema() -> Schema {
    Schema::new(Query, Mutation)
}
