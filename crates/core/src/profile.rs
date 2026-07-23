use crate::{MatrixError, Source};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub source: Source,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl Profile {
    pub fn from_toml(s: &str) -> Result<Profile, MatrixError> {
        toml::from_str(s).map_err(|e| MatrixError::Parse(e.to_string()))
    }
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("serialize profile")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    #[test]
    fn round_trips_profile_toml() {
        let src = "source = \"electron\"\ncapabilities = [\"electron.ipc\", \"electron.tray\"]\n";
        let p = Profile::from_toml(src).expect("parse");
        assert_eq!(p.source, Source::Electron);
        assert_eq!(p.capabilities.len(), 2);
        let out = p.to_toml();
        let p2 = Profile::from_toml(&out).expect("reparse");
        assert_eq!(p2.capabilities, p.capabilities);
    }
}
