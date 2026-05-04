use keyring_core::Entry;

use crate::shared::APPLICATION_NAME_LOWER;

/// Returns the secret from the system key storage, if one exists.
pub fn get_secret(secret: &str) -> anyhow::Result<String> {
    let entry = Entry::new(APPLICATION_NAME_LOWER, secret)?;
    Ok(entry.get_password()?)
}

/// Sets the secret in the system key storage with the given value.
pub fn set_secret(secret: &str, value: &str) -> anyhow::Result<()> {
    let entry = Entry::new(APPLICATION_NAME_LOWER, secret)?;
    Ok(entry.set_password(value)?)
}
