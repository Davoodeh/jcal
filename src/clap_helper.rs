//! A collection of clap helpers.
// this is suboptimal but the has the nicest code for this task without extra "bindings"
// (consts)

use clap::{
    ArgMatches, CommandFactory, FromArgMatches,
    builder::{PossibleValue, PossibleValuesParser, TypedValueParser},
    error::ErrorKind,
};

/// Pairs from strings to values for parsing without ValueEnum trait of clap.
#[derive(Clone, Debug)]
pub struct StaticMap<T>(pub &'static [(&'static str, T)])
where
    T: 'static;

impl<T> StaticMap<T> {
    /// Get all the keys of this hashmap.
    pub fn keys(&self) -> impl Iterator<Item = &'static str> {
        self.0.into_iter().map(|(i, _)| *i)
    }

    /// Get all the values of this hashmap.
    pub fn values(&self) -> impl Iterator<Item = &'static T> {
        self.0.into_iter().map(|(_, i)| i)
    }

    /// Get the value for this key.
    pub fn get(&self, key: &str) -> Option<&'static T> {
        for (k, v) in self.0.into_iter() {
            if *k == key {
                return Some(v);
            }
        }
        None
    }

    /// Get the key ignoring the keys.
    pub fn get_ignore_case(&self, key: &str) -> Option<&'static T> {
        for (k, v) in self.0.into_iter() {
            if k.to_lowercase() == key.to_lowercase() {
                return Some(v);
            }
        }
        None
    }

    /// Get the key for the given value.
    pub fn key_for(&self, value: &'static T) -> Option<&'static str>
    where
        T: PartialEq,
    {
        for (k, v) in self.0.into_iter() {
            if *v == *value {
                return Some(*k);
            }
        }
        None
    }
}

impl<T> TypedValueParser for StaticMap<T>
where
    T: ?Sized + Sync + Send + Clone + 'static,
{
    type Value = T;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let key = PossibleValuesParser::new(self.keys()).parse_ref(cmd, arg, value)?;
        let get_results = if arg.is_some_and(|i| i.is_ignore_case_set()) {
            self.get_ignore_case(&key)
        } else {
            self.get(&key)
        };
        Ok(get_results.unwrap().clone()) // okay unwrap since PossibleValueParser did not throw
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(self.keys().map(|i| PossibleValue::new(i))))
    }
}

/// Extension helper functions for [`CommandFactory`].
pub trait CommandFactoryExt: CommandFactory {
    /// This will throw if the group is not defined.
    fn group_args(group_id: &str) -> Vec<String> {
        Self::command()
            .get_groups()
            .find(|i| i.get_id() == group_id)
            .expect("format group not defined")
            .get_args()
            .map(|i| i.to_string())
            .collect()
    }

    /// Throw an stylish but probably expensive error.
    fn error(kind: ErrorKind, message: impl std::fmt::Display) -> clap::Error {
        Self::command().error(kind, message)
    }
}

impl<T> CommandFactoryExt for T where T: CommandFactory {}

/// Extension helper functions for [`ArgMatches`].
pub trait ArgMatchesExt {
    fn is_explicit(&self, id: &str) -> bool;
}

impl ArgMatchesExt for ArgMatches {
    fn is_explicit(&self, id: &str) -> bool {
        !matches!(
            self.value_source(id),
            None | Some(clap::parser::ValueSource::DefaultValue)
        )
    }
}

/// Replace the clap parse function in no derive environment.
pub trait Parse: CommandFactory + FromArgMatches {
    /// Just like parse in derive feature.
    fn parse() -> Self {
        match Self::from_arg_matches(&Self::command().get_matches()) {
            Ok(v) => v,
            Err(e) => e.exit(),
        }
    }
}

impl<T> Parse for T where T: CommandFactory + FromArgMatches {}
