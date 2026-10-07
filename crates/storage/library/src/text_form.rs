//! The text form of the stored values. A value is written with `Display` and read with `FromStr`.
//! The modules are the `with` targets of the serde attributes of `document.rs`.

use std::fmt::Display;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serializer};

pub(crate) fn serialize<T: Display, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(D::Error::custom)
}

pub(crate) mod option {
    use super::*;

    #[expect(
        clippy::ref_option,
        reason = "the serde with attribute passes the field by reference"
    )]
    pub(crate) fn serialize<T: Display, S: Serializer>(
        value: &Option<T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => serializer.collect_str(value),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|text| text.parse().map_err(D::Error::custom))
            .transpose()
    }
}

pub(crate) mod list {
    use super::*;

    pub(crate) fn serialize<T: Display, S: Serializer>(
        values: &[T],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(values.iter().map(ToString::to_string))
    }

    pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        Vec::<String>::deserialize(deserializer)?
            .iter()
            .map(|text| text.parse().map_err(D::Error::custom))
            .collect()
    }
}
