use std::borrow::Cow;
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub struct Capture<'key, 'value> {
    key: Cow<'key, str>,
    value: Cow<'value, str>,
}

impl<'key, 'value> Capture<'key, 'value> {
    pub fn new(key: impl Into<Cow<'key, str>>, value: impl Into<Cow<'value, str>>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn into_owned(self) -> Capture<'static, 'static> {
        Capture {
            key: self.key.to_string().into(),
            value: self.value.to_string().into(),
        }
    }
}

/// Captured params and wildcards
#[derive(Debug, Default)]
pub struct Captures<'keys, 'values> {
    pub(crate) params: Vec<Capture<'keys, 'values>>,
    pub(crate) wildcard: Option<Cow<'values, str>>,
}

impl<'keys, 'values> Captures<'keys, 'values> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_owned(self) -> Captures<'static, 'static> {
        Captures {
            params: self.params.into_iter().map(|c| c.into_owned()).collect(),
            wildcard: self.wildcard.map(|c| c.to_string().into()),
        }
    }

    pub fn params(&self) -> &[Capture] {
        &self.params[..]
    }

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
