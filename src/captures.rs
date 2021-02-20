use std::ops::Deref;

#[derive(Debug, Default)]
pub struct Captures(pub(crate) Vec<(String, String)>, pub(crate) Option<String>);
impl Captures {
    pub fn wildcard(&self) -> Option<&str> {
        self.1.as_deref()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find_map(|(k, v)| if k == key { Some(&**v) } else { None })
    }
}

impl Deref for Captures {
    type Target = Vec<(String, String)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
