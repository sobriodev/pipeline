//! Helper functions to deal with YAML objects.
//!
//! The module provides a set of free functions to deal with YAML objects which are not a part of
//! `serde_yaml` library but yet useful in terms of this crate.

use crate::error::Pipeline;
use crate::error::Result;
use serde_yaml::{Mapping, Sequence, Value};
use std::ops::ControlFlow;

/// Trait for converting a generic YAML value into an underlying constituent.
pub trait FromYaml<'a> {
    /// A type which is returned as an effect of the conversion.
    type Output;

    /// Parse a YAML value.
    fn parse(value: &'a Value) -> Option<Self::Output>;

    /// Obtain type descriptor for debug purposes.
    fn type_str() -> &'static str;

    /// Convert a YAML value into a desired type.
    ///
    /// # Errors
    /// The function returns an error if the value cannot be represented as the desired type.
    fn try_from(value: &'a Value) -> Result<Self::Output> {
        match Self::parse(value) {
            Some(cv) => Ok(cv),
            None => Err(Pipeline::new_debug(
                &format!(
                    "Could not parse requested yaml value as {}",
                    Self::type_str()
                ),
                &format!("Input object: {:?}", value),
            )),
        }
    }
}

// Impl block generator for types which are obtained by reference
macro_rules! impl_from_yaml_ref {
    ($type:ty) => {
        impl<'a> FromYaml<'a> for $type {
            type Output = &'a Self;

            fn parse(value: &'a Value) -> Option<Self::Output> {
                paste::paste! { value.[<as_ $type:lower>]() }
            }

            fn type_str() -> &'static str {
                stringify!($type)
            }
        }
    };
}

impl_from_yaml_ref!(str);
impl_from_yaml_ref!(Mapping);
impl_from_yaml_ref!(Sequence);

// Impl block generator for primitive types which are copied rather than referenced
macro_rules! impl_from_yaml_cp {
    ($type:ty) => {
        impl<'a> FromYaml<'a> for $type {
            type Output = Self;

            fn parse(value: &'a Value) -> Option<Self::Output> {
                paste::paste! { value.[<as_ $type:lower>]() }
            }

            fn type_str() -> &'static str {
                concat!("$", stringify!($type))
            }
        }
    };
}

impl_from_yaml_cp!(bool);
impl_from_yaml_cp!(i64);
impl_from_yaml_cp!(u64);
impl_from_yaml_cp!(f64);

/// Obtain YAML value by a path.
///
/// The path comprises a specified number of keys separated by a dot character e.g. `key.key2.key3`.
/// Sequence indices are not supported at the moment (each key must be linked to a YAML map).
///
/// # Errors
/// The function returns an error in case specified path was not found inside an input object.
pub fn get_value_by_path<'a>(value: &'a Value, path: &str) -> Result<&'a Value> {
    let cf = path.split('.').try_fold(value, |acc, key| match acc {
        Value::Mapping(map) => {
            let value_from_str = Value::String(key.to_string());
            match map.get(&value_from_str) {
                Some(value) => ControlFlow::Continue(value),
                None => ControlFlow::Break(()),
            }
        }
        _ => ControlFlow::Break(()),
    });

    match cf {
        ControlFlow::Continue(value) => Ok(value),
        ControlFlow::Break(_) => Err(Pipeline::new_debug(
            &format!("Path `{}` was not found within the input object", path),
            &format!("Input object: {:?}", value),
        )),
    }
}

/// Obtain a YAML value with a specific type.
///
/// The function obtains a value similarly to [`get_value_by_path`] with additional type conversion
/// afterwards.
///
/// Following conversions are supported at the moment:
///  - bool
///  - i64
///  - u64
///  - f64
///  - &str
///  - &Mapping
///  - &Sequence
///
/// # Errors
/// The function returns an error in case specified path was not found inside an input object
/// or obtained value cannot be casted to a desired type.
pub fn get_typed_value_by_path<'a, T>(value: &'a Value, path: &str) -> Result<T::Output>
where
    T: ?Sized + FromYaml<'a>,
{
    let v = get_value_by_path(value, path)?;
    T::try_from(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    /* ------------------ */
    /* ---- Fixtures ---- */
    /* ------------------ */

    #[fixture]
    fn test_yaml() -> Value {
        serde_yaml::from_str(
            r#"
            name: "John Doe"
            adult: true
            age: 22
            score: 214.67
            rank_delta: -10
            cars_owned:
                - name: "Ford Mustang"
                  age: 5
                  last_inspection:
                    date: "2020-01-05"
        "#,
        )
        .unwrap()
    }

    /* -------------------------- */
    /* ---- Test definitions ---- */
    /* -------------------------- */

    #[rstest]
    fn get_value_by_path_returns_error_when_empty_path_is_passed(test_yaml: Value) {
        assert!(get_value_by_path(&test_yaml, "").is_err());
    }

    #[rstest]
    #[case(".")]
    #[case("..")]
    #[case(".key")]
    #[case("key1.key2.")]
    fn get_value_by_path_returns_error_when_invalid_path_is_passed(
        #[case] path: &str,
        test_yaml: Value,
    ) {
        assert!(get_value_by_path(&test_yaml, path).is_err());
    }

    #[rstest]
    #[case("invalid.invalid")]
    #[case("name.invalid")]
    #[case("cars_owned.invalid")]
    // Sequence indices not supported
    #[case("cars_owned.0.name")]
    #[case("cars_owned.0.last_inspection")]
    fn get_value_by_path_returns_error_when_non_existing_path_is_passed(
        #[case] path: &str,
        test_yaml: Value,
    ) {
        assert!(get_value_by_path(&test_yaml, path).is_err());
    }

    #[rstest]
    #[case(&test_yaml(), "name")]
    #[case(&test_yaml(), "cars_owned")]
    #[case(&test_yaml()["cars_owned"][0], "name")]
    #[case(&test_yaml()["cars_owned"][0], "age")]
    #[case(&test_yaml()["cars_owned"][0], "last_inspection")]
    #[case(&test_yaml()["cars_owned"][0], "last_inspection.date")]
    fn get_value_by_path_returns_reference_when_existing_path_is_passed(
        #[case] input_yml: &Value,
        #[case] path: &str,
    ) {
        get_value_by_path(input_yml, path).unwrap();
    }

    #[rstest]
    fn get_typed_value_by_path_returns_error_when_invalid_type_requested(test_yaml: Value) {
        assert!(get_typed_value_by_path::<bool>(&test_yaml, "age").is_err());
        assert!(get_typed_value_by_path::<i64>(&test_yaml, "adult").is_err());
        assert!(get_typed_value_by_path::<u64>(&test_yaml, "adult").is_err());
        assert!(get_typed_value_by_path::<f64>(&test_yaml, "adult").is_err());
        assert!(get_typed_value_by_path::<str>(&test_yaml, "age").is_err());
        assert!(get_typed_value_by_path::<Mapping>(&test_yaml, "name").is_err());
        assert!(get_typed_value_by_path::<Sequence>(&test_yaml, "name").is_err());
    }

    #[rstest]
    fn get_typed_value_by_path_valid_value_returned_when_bool_requested(test_yaml: Value) {
        assert_eq!(
            test_yaml["adult"],
            get_typed_value_by_path::<bool>(&test_yaml, "adult").unwrap()
        );
    }

    #[rstest]
    fn get_typed_value_by_path_valid_value_returned_when_i64_requested(test_yaml: Value) {
        assert_eq!(
            test_yaml["rank_delta"],
            get_typed_value_by_path::<i64>(&test_yaml, "rank_delta").unwrap()
        );
    }

    #[rstest]
    fn get_typed_value_by_path_valid_value_returned_when_u64_requested(test_yaml: Value) {
        assert_eq!(
            test_yaml["age"],
            get_typed_value_by_path::<u64>(&test_yaml, "age").unwrap()
        );
    }

    #[rstest]
    fn get_typed_value_by_path_valid_value_returned_when_f64_requested(test_yaml: Value) {
        assert_eq!(
            test_yaml["score"],
            get_typed_value_by_path::<f64>(&test_yaml, "score").unwrap()
        );
    }

    #[rstest]
    fn get_typed_value_by_path_valid_value_returned_when_str_requested(test_yaml: Value) {
        assert_eq!(
            test_yaml["name"],
            get_typed_value_by_path::<str>(&test_yaml, "name").unwrap()
        );
    }

    #[rstest]
    fn get_typed_value_by_path_valid_value_returned_when_mapping_requested(test_yaml: Value) {
        assert_eq!(
            test_yaml["cars_owned"][0]["last_inspection"]
                .as_mapping()
                .unwrap(),
            get_typed_value_by_path::<Mapping>(&test_yaml["cars_owned"][0], "last_inspection")
                .unwrap()
        );
    }

    #[rstest]
    fn get_typed_value_by_path_valid_value_returned_when_sequence_requested(test_yaml: Value) {
        assert_eq!(
            test_yaml["cars_owned"].as_sequence().unwrap(),
            get_typed_value_by_path::<Sequence>(&test_yaml, "cars_owned").unwrap()
        );
    }
}
