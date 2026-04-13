use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[cfg(target_os = "macos")]
use std::process::Command;

use crate::error::AppError;

pub const SECRET_SERVICE_NAME: &str = "com.cxymds.itermmcptools";

pub trait SecretStore: Send + Sync {
    fn set_secret(&self, locator: &str, secret: &str) -> Result<(), AppError>;
    fn get_secret(&self, locator: &str) -> Result<String, AppError>;
    fn delete_secret(&self, locator: &str) -> Result<(), AppError>;
}

#[derive(Debug, Default)]
pub struct SystemSecretStore;

impl SecretStore for SystemSecretStore {
    fn set_secret(&self, locator: &str, secret: &str) -> Result<(), AppError> {
        #[cfg(target_os = "macos")]
        {
            run_security(
                &[
                    "add-generic-password",
                    "-U",
                    "-s",
                    SECRET_SERVICE_NAME,
                    "-a",
                    locator,
                    "-w",
                    secret,
                    "login.keychain-db",
                ],
                locator,
            )
        }

        #[cfg(not(target_os = "macos"))]
        {
            let entry = keyring::Entry::new(SECRET_SERVICE_NAME, locator)
                .map_err(|error| AppError::SecretStore(error.to_string()))?;
            entry
                .set_password(secret)
                .map_err(|error| AppError::SecretStore(error.to_string()))
        }
    }

    fn get_secret(&self, locator: &str) -> Result<String, AppError> {
        #[cfg(target_os = "macos")]
        {
            run_security_output(
                &[
                    "find-generic-password",
                    "-w",
                    "-s",
                    SECRET_SERVICE_NAME,
                    "-a",
                    locator,
                ],
                locator,
            )
        }

        #[cfg(not(target_os = "macos"))]
        {
            let entry = keyring::Entry::new(SECRET_SERVICE_NAME, locator)
                .map_err(|error| AppError::SecretStore(error.to_string()))?;
            entry
                .get_password()
                .map_err(|error| AppError::SecretStore(error.to_string()))
        }
    }

    fn delete_secret(&self, locator: &str) -> Result<(), AppError> {
        #[cfg(target_os = "macos")]
        {
            run_security(
                &[
                    "delete-generic-password",
                    "-s",
                    SECRET_SERVICE_NAME,
                    "-a",
                    locator,
                ],
                locator,
            )
        }

        #[cfg(not(target_os = "macos"))]
        {
            let entry = keyring::Entry::new(SECRET_SERVICE_NAME, locator)
                .map_err(|error| AppError::SecretStore(error.to_string()))?;
            entry
                .delete_credential()
                .map_err(|error| AppError::SecretStore(error.to_string()))
        }
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

    fn delete_secret(&self, locator: &str) -> Result<(), AppError> {
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStore("memory secret store lock poisoned".to_string()))?
            .remove(locator);
        Ok(())
    }
}

pub fn profile_secret_locator(profile_id: &str) -> String {
    format!("secret://profile/{profile_id}")
}

fn map_security_error(locator: &str, stderr: &str) -> AppError {
    let message = stderr.trim().to_string();
    if message.contains("could not be found in the keychain") {
        AppError::MissingDependency(format!("secret not found for locator {locator}"))
    } else {
        AppError::SecretStore(message)
    }
}

#[cfg(target_os = "macos")]
fn run_security(args: &[&str], locator: &str) -> Result<(), AppError> {
    let output = Command::new("security").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(map_security_error(locator, &String::from_utf8_lossy(&output.stderr)))
    }
}

#[cfg(target_os = "macos")]
fn run_security_output(args: &[&str], locator: &str) -> Result<String, AppError> {
    let output = Command::new("security").args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(map_security_error(
            locator,
            &String::from_utf8_lossy(&output.stderr),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_security_missing_item_to_missing_dependency() {
        let error = map_security_error(
            "secret://profile/test-profile",
            "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain.\n",
        );

        assert!(matches!(error, AppError::MissingDependency(_)));
        assert!(error
            .to_string()
            .contains("secret not found for locator secret://profile/test-profile"));
    }

    #[test]
    fn preserves_security_error_output_for_other_failures() {
        let error = map_security_error("secret://profile/test-profile", "custom failure\n");

        assert!(matches!(error, AppError::SecretStore(_)));
        assert!(error.to_string().contains("custom failure"));
    }
}
