use serde::{Deserialize, Serialize};
use shared::error::{DomainError, DomainResult};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.contains('@') && value.contains('.') {
            Ok(Self(value))
        } else {
            Err(DomainError::Validation(format!(
                "Invalid email format: {}",
                value
            )))
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneNumber(String);

impl PhoneNumber {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.starts_with('+')
            && value.len() >= 8
            && value.len() <= 16
            && value[1..].chars().all(|c| c.is_ascii_digit())
        {
            Ok(Self(value))
        } else {
            Err(DomainError::Validation(format!(
                "Invalid phone number format (E.164 required): {}",
                value
            )))
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Username(String);

impl Username {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.len() >= 3
            && value.len() <= 32
            && value.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            Ok(Self(value))
        } else {
            Err(DomainError::Validation(format!(
                "Invalid username: {}",
                value
            )))
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordHash(pub String);

impl PasswordHash {
    pub fn new(value: String) -> DomainResult<Self> {
        if !value.is_empty() {
            Ok(Self(value))
        } else {
            Err(DomainError::Validation(
                "Password hash cannot be empty".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(Email::new("test@example.com".to_string()).is_ok());
        assert!(Email::new("invalid-email".to_string()).is_err());
    }

    #[test]
    fn test_phone_validation() {
        assert!(PhoneNumber::new("+573001234567".to_string()).is_ok());
        assert!(PhoneNumber::new("3001234567".to_string()).is_err());
        assert!(PhoneNumber::new("+1234".to_string()).is_err());
    }

    #[test]
    fn test_username_validation() {
        assert!(Username::new("julio_dev".to_string()).is_ok());
        assert!(Username::new("ju".to_string()).is_err());
        assert!(Username::new("invalid name!".to_string()).is_err());
    }
}
