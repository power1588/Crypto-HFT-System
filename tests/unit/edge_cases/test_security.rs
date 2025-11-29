use crypto_hft::security::{SecureApiKey, ApiKeyError, ApiKeyManager};

#[test]
fn test_secure_api_key_masking() {
    let key = SecureApiKey::new("abcdefghijklmnopqrstuvwxyz123456".to_string());
    let masked = key.mask();
    assert_eq!(masked, "abcd...3456");
    assert!(!masked.contains("efghijklmnopqrstuvwxyz"));
}

#[test]
fn test_secure_api_key_validation_too_short() {
    let key = SecureApiKey::new("short".to_string());
    assert!(key.validate().is_err());
}

#[test]
fn test_secure_api_key_validation_demo_key() {
    let key = SecureApiKey::new("demo_api_key".to_string());
    assert!(key.validate().is_err());
    
    let key2 = SecureApiKey::new("test_api_key".to_string());
    assert!(key2.validate().is_err());
}

#[test]
fn test_secure_api_key_validation_valid() {
    let key = SecureApiKey::new("a".repeat(16));
    assert!(key.validate().is_ok());
}

#[test]
fn test_api_key_manager_add_invalid() {
    let mut manager = ApiKeyManager::new();
    let key = SecureApiKey::new("short".to_string());
    assert!(manager.add_key("test".to_string(), key).is_err());
}

#[test]
fn test_api_key_manager_add_valid() {
    let mut manager = ApiKeyManager::new();
    let key = SecureApiKey::new("a".repeat(16));
    assert!(manager.add_key("test".to_string(), key).is_ok());
    assert!(manager.get_key("test").is_some());
}

#[test]
fn test_api_key_manager_validate_all() {
    let mut manager = ApiKeyManager::new();
    let key1 = SecureApiKey::new("a".repeat(16));
    let key2 = SecureApiKey::new("short".to_string());
    
    manager.add_key("key1".to_string(), key1).unwrap();
    manager.add_key("key2".to_string(), key2).unwrap_err(); // Should fail
    
    // Only valid key should be added
    assert!(manager.get_key("key1").is_some());
    assert!(manager.get_key("key2").is_none());
}

