//! Newtype wrappers for type-safe temperature celsius and fahrenheit readings.
//!
//! These newtype wrappers are created so that the typechecker helps us keep Celsius and Fahrenheit
//! values separate (i.e., we don't accidentally use a Celsius reading where we expect a Fahrenheit
//! reading, and vice versa.)
//!
//! ```
//! # use temperature_app::temperature::{Celsius, Fahrenheit};
//! let degrees_celsius: Celsius = 0.0.into();
//! let degrees_fahrenheit: Fahrenheit = degrees_celsius.into();
//! assert_eq!(degrees_fahrenheit.value(), 32.0);
//! ```
//!
//! ```compile_fail
//! # use temperature_app::temperature::{Celsius, Fahrenheit};
//! let degrees_celsius: Celsius = 1.0.into();
//! let degrees_fahrenheit: Fahrenheit = 35.0.into();
//! // Shouldn't compile...
//! let result = degress_fahrenheit + degress_celsius;
//! ```

/// Temperature, in degrees celsius
#[derive(juniper::GraphQLScalarValue, Clone, Copy)]
pub struct Celsius(f64);

impl Celsius {
    /// Get the f64 value back from this Celsius measurement.
    pub fn value(&self) -> f64 {
        return self.0;
    }
}

impl From<f64> for Celsius {
    /// Mark an f64 value as being a reading in Celsius.
    fn from(val: f64) -> Self {
        Celsius(val)
    }
}

impl From<Celsius> for f64 {
    /// Get the f64 value back from this Celsius measurement.
    fn from(val: Celsius) -> Self {
        val.0
    }
}

impl std::ops::Add for Celsius {
    type Output = Self;

    /// Add two Celsius readings together. This makes sense when we have adjustments stored as
    /// degrees celsius.
    fn add(self, rhs: Celsius) -> Self::Output {
        Celsius(self.0 + rhs.0)
    }
}

/// Temperature, in degrees fahrenheit
#[derive(juniper::GraphQLScalarValue, Clone, Copy)]
pub struct Fahrenheit(f64);

impl Fahrenheit {
    /// Get the f64 value back from this Fahrenheit measurement.
    pub fn value(&self) -> f64 {
        return self.0;
    }
}

impl From<f64> for Fahrenheit {
    /// Mark an f64 value as being a reading in Fahrenheit.
    fn from(val: f64) -> Self {
        Fahrenheit(val)
    }
}

impl From<Celsius> for Fahrenheit {
    /// Convert a Celsius value to a Fahrenheit value.
    fn from(val: Celsius) -> Self {
        let val = val.0 * 1.8 + 32.0;
        Fahrenheit(val)
    }
}
