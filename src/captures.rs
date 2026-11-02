use smartcow::SmartCow;
use std::{
    borrow::Cow,
    iter::FromIterator,
    ops::{Deref, DerefMut},
};

/// An individual key-value pair
#[derive(Debug, Default)]
pub struct Capture<'key, 'value> {
    key: SmartCow<'key>,
    value: SmartCow<'value>,
}

impl<'key, 'value> Capture<'key, 'value> {
    /// Build a new Capture from the provided key and value. Passing a
    /// &str here is preferable, but a String will also work.
    pub fn new(key: impl Into<Cow<'key, str>>, value: impl Into<Cow<'value, str>>) -> Self {
        Self {
            key: key.into().into(),
            value: value.into().into(),
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
            key: self.key.into_owned(),
            value: self.value.into_owned(),
        }
    }
}

/// Captured params and a wildcard
#[derive(Debug, Default)]
pub struct Captures<'keys, 'values> {
    pub(crate) params: Vec<Capture<'keys, 'values>>,
    pub(crate) wildcard: Option<SmartCow<'values>>,
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
            wildcard: self.wildcard.map(SmartCow::into_owned),
        }
    }

    /// returns a slice of captures
    pub fn params(&self) -> &[Capture] {
        &self.params[..]
    }

    /// set the captured wildcard to the provided &str or
    /// String. Prefer passing a &str if available.
    pub fn set_wildcard(&mut self, wildcard: impl Into<Cow<'values, str>>) {
        self.wildcard = Some(wildcard.into().into());
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

    /// Combine two captures
    pub fn append(&mut self, mut captures: Captures<'keys, 'values>) {
        self.params.append(&mut captures.params);
        self.wildcard = captures.wildcard;
    }

    /// Iterate over params as str pairs
    pub fn iter(&self) -> Iter<'_, '_, '_> {
        self.into()
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

impl<'pair, 'key: 'pair, 'value: 'pair> From<&'pair (&'key str, &'value str)>
    for Capture<'key, 'value>
{
    fn from(kv: &'pair (&'key str, &'value str)) -> Self {
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

impl<'keys, 'values, I: Into<Capture<'keys, 'values>>> FromIterator<I>
    for Captures<'keys, 'values>
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self {
            params: iter.into_iter().map(Into::into).collect(),
            wildcard: None,
        }
    }
}

impl<'keys, 'values, I: Into<Capture<'keys, 'values>>> Extend<I> for Captures<'keys, 'values> {
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        self.params.extend(iter.into_iter().map(Into::into));
    }
}

impl<'keys, 'values> IntoIterator for Captures<'keys, 'values> {
    type Item = Capture<'keys, 'values>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.params.into_iter()
    }
}

#[derive(Debug)]
pub struct Iter<'captures: 'keys + 'values, 'keys, 'values>(
    std::slice::Iter<'captures, Capture<'keys, 'values>>,
);
impl<'captures: 'keys + 'values, 'keys, 'values> Iterator for Iter<'captures, 'keys, 'values> {
    type Item = (&'keys str, &'values str);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|c| (c.name(), c.value()))
    }
}

impl<'captures: 'keys + 'values, 'keys, 'values> From<&'captures Captures<'keys, 'values>>
    for Iter<'captures, 'keys, 'values>
{
    fn from(value: &'captures Captures<'keys, 'values>) -> Self {
        Iter(value.params.iter())
    }
}

impl<'captures: 'keys + 'values, 'keys, 'values> IntoIterator
    for &'captures Captures<'keys, 'values>
{
    type Item = (&'keys str, &'values str);

    type IntoIter = Iter<'captures, 'keys, 'values>;

    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}
