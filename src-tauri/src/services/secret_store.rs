use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::error::AppError;

pub const SECRET_SERVICE_NAME: &str = "com.cxymds.itermmcptools";

pub trait SecretStore: Send + Sync {
    fn set_secret(&self, locator: &str, secret: &str) -> Result<(), AppError>;
    fn get_secret(&self, locator: &str) -> Result<String, AppError>;
}

#[derive(Debug, Default)]
pub struct SystemSecretStore;

impl SecretStore for SystemSecretStore {
    fn set_secret(&self, locator: &str, secret: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new(SECRET_SERVICE_NAME, locator)
            .map_err(|error| AppError::SecretStore(error.to_string()))?;
        entry
            .set_password(secret)
            .map_err(|error| AppError::SecretStore(error.to_string()))
    }

    fn get_secret(&self, locator: &str) -> Result<String, AppError> {
        let entry = keyring::Entry::new(SECRET_SERVICE_NAME, locator)
            .map_err(|error| AppError::SecretStore(error.to_string()))?;
        entry
            .get_password()
            .map_err(|error| AppError::SecretStore(error.to_string()))
    }
}

#[derive(Debug, Default, Clone)]
pub struct MemorySecretStore {
    secrets: Arc<Mutex<HashMap<String, String>>>,
}

impl MemorySecretStore {
    pub fn get_secret(&self, locator: &str) -> Option<String> {
        self.secrets
            .lock()
            .ok()
            .and_then(|map| map.get(locator).cloned())
    }
}

impl SecretStore for MemorySecretStore {
    fn set_secret(&self, locator: &str, secret: &str) -> Result<(), AppError> {
        let mut map = self
            .secrets
            .lock()
            .map_err(|_| AppError::SecretStore("memory secret store lock poisoned".to_string()))?;
        map.insert(locator.to_string(), secret.to_string());
        Ok(())
    }

    fn get_secret(&self, locator: &str) -> Result<String, AppError> {
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStore("memory secret store lock poisoned".to_string()))?
            .get(locator)
            .cloned()
            .ok_or_else(|| {
                AppError::MissingDependency(format!("secret not found for locator {locator}"))
            })
    }
}

pub fn profile_secret_locator(profile_id: &str) -> String {
    format!("secret://profile/{profile_id}")
}
