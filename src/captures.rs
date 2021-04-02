use std::{
    borrow::Cow,
    iter::FromIterator,
    ops::{Deref, DerefMut},
};

/// An individual key-value pair
#[derive(Debug, Default)]
pub struct Capture<'key, 'value> {
    key: Cow<'key, str>,
    value: Cow<'value, str>,
}

impl<'key, 'value> Capture<'key, 'value> {
    /// Build a new Capture from the provided key and value. Passing a
    /// &str here is preferable, but a String will also work.
    pub fn new(key: impl Into<Cow<'key, str>>, value: impl Into<Cow<'value, str>>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    /// returns the name of this capture
    pub fn name(&self) -> &str {
        &self.key
    }

    /// returns the value of this capture
    pub fn value(&self) -> &str {
        &self.value
    }

    /// transforms this potentially-borrowed Capture into a 'static
    /// capture that can outlive the source data. This allocates new
    /// strings if needed, and should be avoided unless necessary for
    /// a particular application
    pub fn into_owned(self) -> Capture<'static, 'static> {
        Capture {
            key: self.key.to_string().into(),
            value: self.value.to_string().into(),
        }
    }
}

/// Captured params and a wildcard
#[derive(Debug, Default)]
pub struct Captures<'keys, 'values> {
    pub(crate) params: Vec<Capture<'keys, 'values>>,
    pub(crate) wildcard: Option<Cow<'values, str>>,
}

impl<'keys, 'values> Captures<'keys, 'values> {
    /// Builds a new empty Captures
    pub fn new() -> Self {
        Self::default()
    }

    /// Transforms this Captures into a 'static Captures which can
    /// outlive the source data. This allocates new strings if needed,
    /// and should be avoided unless necessary for a particular
    /// application
    pub fn into_owned(self) -> Captures<'static, 'static> {
        Captures {
            params: self.params.into_iter().map(|c| c.into_owned()).collect(),
            wildcard: self.wildcard.map(|c| c.to_string().into()),
        }
    }

    /// returns a slice of captures
    pub fn params(&self) -> &[Capture] {
        &self.params[..]
    }

    /// set the captured wildcard to the provided &str or
    /// String. Prefer passing a &str if available.
    pub fn set_wildcard(&mut self, wildcard: impl Into<Cow<'values, str>>) {
        self.wildcard = Some(wildcard.into());
    }

    /// returns what the * wildcard matched, if any
    pub fn wildcard(&self) -> Option<&str> {
        self.wildcard.as_deref()
    }

    /// checks the list of params for a matching key
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.iter().find_map(|capture| {
            if capture.key == key {
                Some(&*capture.value)
            } else {
                None
            }
        })
    }

    /// Add the provided Capture (or capture-like) to the end of the params
    pub fn push(&mut self, capture: impl Into<Capture<'keys, 'values>>) {
        self.params.push(capture.into());
    }
}

impl<'keys, 'values> Deref for Captures<'keys, 'values> {
    type Target = Vec<Capture<'keys, 'values>>;

    fn deref(&self) -> &Self::Target {
        &self.params
    }
}

impl<'keys, 'values> DerefMut for Captures<'keys, 'values> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.params
    }
}

impl<'key, 'value> From<(&'key str, &'value str)> for Capture<'key, 'value> {
    fn from(kv: (&'key str, &'value str)) -> Self {
        Self {
            key: kv.0.into(),
            value: kv.1.into(),
        }
    }
}

impl<'keys, 'values, F> From<F> for Captures<'keys, 'values>
where
    F: IntoIterator<Item = (&'keys str, &'values str)>,
{
    fn from(f: F) -> Self {
        f.into_iter().collect()
    }
}

impl<'keys, 'values> FromIterator<(&'keys str, &'values str)> for Captures<'keys, 'values> {
    fn from_iter<T: IntoIterator<Item = (&'keys str, &'values str)>>(iter: T) -> Self {
        Self {
            params: iter.into_iter().map(Into::into).collect(),
            wildcard: None,
        }
    }
}

impl<'keys, 'values> Extend<(&'keys str, &'values str)> for Captures<'keys, 'values> {
    fn extend<T: IntoIterator<Item = (&'keys str, &'values str)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.params.push(Capture::new(k, v));
        }
    }
}
