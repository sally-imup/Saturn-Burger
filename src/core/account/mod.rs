//! Local/offline player profiles.
//!
//! The offline UUID is calculated using the same name-based MD5 algorithm used by
//! Java's `UUID.nameUUIDFromBytes` with the `OfflinePlayer:<name>` input.
//! This does not authenticate a player with Microsoft/Mojang services.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

const STORE_FILE: &str = "local_profiles.json";
const STORE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalProfile {
    pub username: String,
    pub offline_uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountData {
    schema_version: u32,
    selected_profile: Option<String>,
    profiles: Vec<LocalProfile>,
}

impl Default for AccountData {
    fn default() -> Self {
        Self {
            schema_version: STORE_SCHEMA_VERSION,
            selected_profile: None,
            profiles: Vec::new(),
        }
    }
}

pub struct LocalAccountStore {
    path: PathBuf,
    data: AccountData,
}

impl LocalAccountStore {
    /// Loads the account store or creates an empty in-memory store if it does not exist.
    pub fn load(data_directory: &Path) -> Result<Self, String> {
        let path = data_directory.join("accounts").join(STORE_FILE);
        let backup_path = path.with_extension("json.bak");
        if !path.exists() && backup_path.exists() {
            fs::rename(&backup_path, &path)
                .map_err(|error| format!("Could not restore the local profile backup: {error}"))?;
        }
        let data = if path.exists() {
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("Could not read local profiles: {error}"))?;
            let parsed: AccountData = serde_json::from_str(&contents)
                .map_err(|error| format!("Local profile file is invalid JSON: {error}"))?;
            if parsed.schema_version != STORE_SCHEMA_VERSION {
                return Err(format!(
                    "Unsupported local profile schema version: {}",
                    parsed.schema_version
                ));
            }
            validate_loaded_data(parsed)?
        } else {
            AccountData::default()
        };

        let store = Self { path, data };
        if !store.path.exists() {
            store.save()?;
        }
        Ok(store)
    }

    pub fn profiles(&self) -> &[LocalProfile] {
        &self.data.profiles
    }

    pub fn selected_profile(&self) -> Option<&LocalProfile> {
        let selected = self.data.selected_profile.as_deref()?;
        self.data
            .profiles
            .iter()
            .find(|profile| profile.offline_uuid == selected)
    }

    pub fn selected_uuid(&self) -> Option<&str> {
        self.data.selected_profile.as_deref()
    }

    pub fn create_profile(&mut self, username: &str) -> Result<(), String> {
        let username = normalize_and_validate_username(username)?;
        if self
            .data
            .profiles
            .iter()
            .any(|profile| profile.username.eq_ignore_ascii_case(&username))
        {
            return Err("A profile with that username already exists.".to_owned());
        }

        let profile = LocalProfile {
            offline_uuid: offline_uuid(&username),
            username,
        };
        let previous = self.data.clone();
        self.data.selected_profile = Some(profile.offline_uuid.clone());
        self.data.profiles.push(profile);
        self.persist_or_rollback(previous)
    }

    pub fn select_profile(&mut self, offline_uuid: &str) -> Result<(), String> {
        if !self
            .data
            .profiles
            .iter()
            .any(|profile| profile.offline_uuid == offline_uuid)
        {
            return Err("The selected profile no longer exists.".to_owned());
        }
        let previous = self.data.clone();
        self.data.selected_profile = Some(offline_uuid.to_owned());
        self.persist_or_rollback(previous)
    }

    /// Renaming changes the offline UUID because Minecraft derives it from the username.
    pub fn rename_selected(&mut self, username: &str) -> Result<(), String> {
        let username = normalize_and_validate_username(username)?;
        let selected = self
            .data
            .selected_profile
            .clone()
            .ok_or_else(|| "Select a profile before renaming it.".to_owned())?;

        if self.data.profiles.iter().any(|profile| {
            profile.offline_uuid != selected
                && profile.username.eq_ignore_ascii_case(&username)
        }) {
            return Err("A profile with that username already exists.".to_owned());
        }

        let previous = self.data.clone();
        let profile = self
            .data
            .profiles
            .iter_mut()
            .find(|profile| profile.offline_uuid == selected)
            .ok_or_else(|| "The selected profile no longer exists.".to_owned())?;

        let new_uuid = offline_uuid(&username);
        profile.username = username;
        profile.offline_uuid = new_uuid.clone();
        self.data.selected_profile = Some(new_uuid);
        self.persist_or_rollback(previous)
    }

    pub fn delete_selected(&mut self) -> Result<(), String> {
        let selected = self
            .data
            .selected_profile
            .clone()
            .ok_or_else(|| "There is no selected profile to delete.".to_owned())?;
        let previous = self.data.clone();
        let old_len = self.data.profiles.len();
        self.data
            .profiles
            .retain(|profile| profile.offline_uuid != selected);
        if old_len == self.data.profiles.len() {
            return Err("The selected profile no longer exists.".to_owned());
        }
        self.data.selected_profile = self
            .data
            .profiles
            .first()
            .map(|profile| profile.offline_uuid.clone());
        self.persist_or_rollback(previous)
    }

    fn persist_or_rollback(&mut self, previous: AccountData) -> Result<(), String> {
        if let Err(error) = self.save() {
            self.data = previous;
            return Err(error);
        }
        Ok(())
    }

    fn save(&self) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "The account storage path has no parent directory.".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create account directory: {error}"))?;

        let serialized = serde_json::to_vec_pretty(&self.data)
            .map_err(|error| format!("Could not serialize local profiles: {error}"))?;
        let temporary_path = self.path.with_extension("json.tmp");
        let backup_path = self.path.with_extension("json.bak");

        let mut temporary_file = fs::File::create(&temporary_path)
            .map_err(|error| format!("Could not create temporary profile file: {error}"))?;
        temporary_file
            .write_all(&serialized)
            .and_then(|()| temporary_file.sync_all())
            .map_err(|error| format!("Could not write local profiles: {error}"))?;
        drop(temporary_file);

        let had_original = self.path.exists();
        if had_original {
            if backup_path.exists() {
                fs::remove_file(&backup_path)
                    .map_err(|error| format!("Could not replace profile backup: {error}"))?;
            }
            fs::rename(&self.path, &backup_path)
                .map_err(|error| format!("Could not stage existing profile file: {error}"))?;
        }

        if let Err(error) = fs::rename(&temporary_path, &self.path) {
            if had_original {
                let _ = fs::rename(&backup_path, &self.path);
            }
            let _ = fs::remove_file(&temporary_path);
            return Err(format!("Could not commit local profiles: {error}"));
        }

        if had_original {
            let _ = fs::remove_file(backup_path);
        }
        Ok(())
    }
}

fn validate_loaded_data(mut data: AccountData) -> Result<AccountData, String> {
    let mut seen = std::collections::HashSet::new();
    for profile in &data.profiles {
        normalize_and_validate_username(&profile.username)?;
        let expected_uuid = offline_uuid(&profile.username);
        if profile.offline_uuid != expected_uuid {
            return Err(format!(
                "Profile '{}' has an invalid offline UUID. The account file was not modified.",
                profile.username
            ));
        }
        if !seen.insert(profile.username.to_ascii_lowercase()) {
            return Err("The account file contains duplicate usernames.".to_owned());
        }
    }

    let selected_is_valid = data
        .selected_profile
        .as_ref()
        .is_some_and(|selected| data.profiles.iter().any(|profile| &profile.offline_uuid == selected));
    if !selected_is_valid {
        data.selected_profile = data.profiles.first().map(|profile| profile.offline_uuid.clone());
    }
    Ok(data)
}

fn normalize_and_validate_username(username: &str) -> Result<String, String> {
    let username = username.trim();
    if !(3..=16).contains(&username.len()) {
        return Err("Minecraft usernames must contain 3–16 characters.".to_owned());
    }
    if !username
        .bytes()
        .all(|character| character.is_ascii_alphanumeric() || character == b'_')
    {
        return Err("Use only English letters, numbers, and underscores.".to_owned());
    }
    Ok(username.to_owned())
}

/// Generates the Java-compatible offline UUID for a Minecraft username.
pub fn offline_uuid(username: &str) -> String {
    let input = format!("OfflinePlayer:{username}");
    let digest = Md5::digest(input.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest);
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::{LocalAccountStore, offline_uuid};

    fn temporary_directory() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("saturn-account-test-{nonce}"))
    }

    #[test]
    fn offline_uuid_matches_java_name_uuid_algorithm() {
        assert_eq!(
            offline_uuid("Notch"),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(
            offline_uuid("Steve"),
            "5627dd98-e6be-3c21-b8a8-e92344183641"
        );
        assert_ne!(offline_uuid("Steve"), offline_uuid("steve"));
    }

    #[test]
    fn profiles_persist_and_selected_profile_survives_reload() {
        let root = temporary_directory();
        std::fs::create_dir_all(root.join("accounts")).unwrap();

        let mut store = LocalAccountStore::load(&root).unwrap();
        store.create_profile("Steve").unwrap();
        store.create_profile("Alex_2").unwrap();
        let selected = store.selected_profile().unwrap().username.clone();
        assert_eq!(selected, "Alex_2");

        let reloaded = LocalAccountStore::load(&root).unwrap();
        assert_eq!(reloaded.profiles().len(), 2);
        assert_eq!(reloaded.selected_profile().unwrap().username, "Alex_2");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_names_are_rejected_without_case_sensitivity() {
        let root = temporary_directory();
        std::fs::create_dir_all(root.join("accounts")).unwrap();
        let mut store = LocalAccountStore::load(&root).unwrap();
        store.create_profile("Steve").unwrap();
        assert!(store.create_profile("steve").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rename_updates_offline_uuid_and_delete_selects_next_profile() {
        let root = temporary_directory();
        std::fs::create_dir_all(root.join("accounts")).unwrap();
        let mut store = LocalAccountStore::load(&root).unwrap();
        store.create_profile("Steve").unwrap();
        let steve_uuid = store.selected_uuid().unwrap().to_owned();
        store.create_profile("Alex").unwrap();

        store.select_profile(&steve_uuid).unwrap();
        store.rename_selected("Builder").unwrap();
        assert_eq!(store.selected_profile().unwrap().username, "Builder");
        let builder_uuid = offline_uuid("Builder");
        assert_eq!(store.selected_uuid(), Some(builder_uuid.as_str()));

        store.delete_selected().unwrap();
        assert_eq!(store.selected_profile().unwrap().username, "Alex");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_usernames_are_rejected() {
        let root = temporary_directory();
        std::fs::create_dir_all(root.join("accounts")).unwrap();
        let mut store = LocalAccountStore::load(&root).unwrap();
        assert!(store.create_profile("ab").is_err());
        assert!(store.create_profile("not valid").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
