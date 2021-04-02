use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub struct Capture(String, String);
impl Capture {
    pub fn new(key: String, value: String) -> Self {
        Self(key, value)
    }

    pub fn name(&self) -> &str {
        &self.0
    }
    pub fn value(&self) -> &str {
        &self.1
    }
}

/// Captured params and wildcards
#[derive(Debug, Default)]
pub struct Captures {
    pub(crate) params: Vec<Capture>,
    pub(crate) wildcard: Option<String>,
}

impl Captures {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn params(&self) -> &[Capture] {
        &self.params[..]
    }

    pub fn set_wildcard(&mut self, wildcard: String) {
        self.wildcard = Some(wildcard);
    }

    /// returns what the * wildcard matched, if any
    pub fn wildcard(&self) -> Option<&str> {
        self.wildcard.as_deref()
    }

    /// checks the list of params for a matching key
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find_map(|Capture(k, v)| if k == key { Some(&**v) } else { None })
    }
}

impl Deref for Captures {
    type Target = Vec<Capture>;

    fn deref(&self) -> &Self::Target {
        &self.params
    }
}

impl DerefMut for Captures {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.params
    }
}
