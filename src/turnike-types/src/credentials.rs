use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    pub fn masked_username(&self) -> String {
        if self.username.len() <= 4 {
            return "****".to_string();
        }
        let start = &self.username[..2];
        let end = &self.username[self.username.len() - 2..];
        let stars = "*".repeat(self.username.len() - 4);
        format!("{start}{stars}{end}")
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.masked_username())
            .field("password", &"[REDACTED]")
            .finish()
    }
}
