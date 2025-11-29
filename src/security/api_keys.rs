use std::env;
use std::fmt;
use std::str::FromStr;

/// Secure API key wrapper that prevents accidental logging
#[derive(Clone)]
pub struct SecureApiKey {
    key: String,
}

impl SecureApiKey {
    /// Create a new secure API key
    pub fn new(key: String) -> Self {
        Self { key }
    }

    /// Get the API key (use with caution)
    pub fn expose(&self) -> &str {
        &self.key
    }

    /// Load API key from environment variable
    pub fn from_env(var_name: &str) -> Result<Self, ApiKeyError> {
        env::var(var_name)
            .map_err(|_| ApiKeyError::Missing(var_name.to_string()))
            .and_then(|key| {
                if key.is_empty() {
                    Err(ApiKeyError::Empty(var_name.to_string()))
                } else if key == "demo_api_key" || key == "test_api_key" {
                    Err(ApiKeyError::InvalidDemoKey(var_name.to_string()))
                } else {
                    Ok(Self::new(key))
                }
            })
    }

    /// Load API key from environment with fallback (for testnet)
    pub fn from_env_or_testnet(var_name: &str, testnet: bool) -> Self {
        if testnet {
            Self::new("test_api_key".to_string())
        } else {
            Self::from_env(var_name).unwrap_or_else(|_| {
                // Log warning but allow demo key in non-production
                log::warn!(
                    "Using demo API key for {} (not suitable for production)",
                    var_name
                );
                Self::new("demo_api_key".to_string())
            })
        }
    }

    /// Validate API key format
    pub fn validate(&self) -> Result<(), ApiKeyError> {
        if self.key.is_empty() {
            return Err(ApiKeyError::Empty("key".to_string()));
        }
        if self.key == "demo_api_key" || self.key == "test_api_key" {
            return Err(ApiKeyError::InvalidDemoKey("key".to_string()));
        }
        if self.key.len() < 16 {
            return Err(ApiKeyError::TooShort);
        }
        Ok(())
    }

    /// Mask the key for logging (shows only first 4 and last 4 characters)
    pub fn mask(&self) -> String {
        if self.key.len() <= 8 {
            "****".to_string()
        } else {
            format!("{}...{}", &self.key[..4], &self.key[self.key.len() - 4..])
        }
    }
}

impl fmt::Debug for SecureApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecureApiKey")
            .field("key", &self.mask())
            .finish()
    }
}

impl fmt::Display for SecureApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mask())
    }
}

impl FromStr for SecureApiKey {
    type Err = ApiKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(ApiKeyError::Empty("key".to_string()))
        } else {
            Ok(Self::new(s.to_string()))
        }
    }
}

/// API Key Manager for managing multiple API keys
pub struct ApiKeyManager {
    keys: std::collections::HashMap<String, SecureApiKey>,
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    /// Add an API key
    pub fn add_key(&mut self, name: String, key: SecureApiKey) -> Result<(), ApiKeyError> {
        key.validate()?;
        self.keys.insert(name, key);
        Ok(())
    }

    /// Get an API key
    pub fn get_key(&self, name: &str) -> Option<&SecureApiKey> {
        self.keys.get(name)
    }

    /// Load API keys from environment variables
    pub fn load_from_env(&mut self, mappings: &[(&str, &str)]) -> Result<(), ApiKeyError> {
        for (name, env_var) in mappings {
            let key = SecureApiKey::from_env(env_var)?;
            self.add_key(name.to_string(), key)?;
        }
        Ok(())
    }

    /// Validate all keys
    pub fn validate_all(&self) -> Result<(), Vec<ApiKeyError>> {
        let mut errors = Vec::new();
        for (name, key) in &self.keys {
            if let Err(e) = key.validate() {
                errors.push(ApiKeyError::ValidationFailed(name.clone(), Box::new(e)));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// API Key errors
#[derive(Debug, Clone)]
pub enum ApiKeyError {
    Missing(String),
    Empty(String),
    InvalidDemoKey(String),
    TooShort,
    ValidationFailed(String, Box<ApiKeyError>),
}

impl fmt::Display for ApiKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiKeyError::Missing(var) => {
                write!(f, "API key environment variable '{}' is not set", var)
            }
            ApiKeyError::Empty(var) => write!(f, "API key environment variable '{}' is empty", var),
            ApiKeyError::InvalidDemoKey(var) => {
                write!(
                    f,
                    "API key '{}' contains demo/test key (not suitable for production)",
                    var
                )
            }
            ApiKeyError::TooShort => write!(f, "API key is too short (minimum 16 characters)"),
            ApiKeyError::ValidationFailed(name, err) => {
                write!(f, "Validation failed for key '{}': {}", name, err)
            }
        }
    }
}

impl std::error::Error for ApiKeyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_api_key_masking() {
        let key = SecureApiKey::new("abcdefghijklmnopqrstuvwxyz".to_string());
        let masked = key.mask();
        assert_eq!(masked, "abcd...wxyz");
    }

    #[test]
    fn test_secure_api_key_short_masking() {
        let key = SecureApiKey::new("short".to_string());
        let masked = key.mask();
        assert_eq!(masked, "****");
    }

    #[test]
    fn test_secure_api_key_validation() {
        let key = SecureApiKey::new("a".repeat(16));
        assert!(key.validate().is_ok());

        let short_key = SecureApiKey::new("short".to_string());
        assert!(short_key.validate().is_err());

        let demo_key = SecureApiKey::new("demo_api_key".to_string());
        assert!(demo_key.validate().is_err());
    }

    #[test]
    fn test_api_key_manager() {
        let mut manager = ApiKeyManager::new();
        let key = SecureApiKey::new("a".repeat(16));
        assert!(manager.add_key("test".to_string(), key).is_ok());
        assert!(manager.get_key("test").is_some());
    }
}
