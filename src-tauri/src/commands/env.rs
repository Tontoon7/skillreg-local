use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

const ORG_VARIABLES_FILE: &str = "variables.env";
const ORG_INDEX_FILE: &str = "index.json";
const FALLBACK_FILE_STORAGE: &str = "fallback_file";
const CREDENTIAL_SERVICE: &str = "skillreg";

fn env_root() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".skillreg").join("env")
}

fn env_dir(org: &str) -> PathBuf {
    env_root().join(org)
}

fn env_file(org: &str, skill: &str) -> PathBuf {
    env_dir(org).join(format!("{}.env", skill))
}

fn normalize_key(key: &str) -> String {
    key.trim().to_uppercase()
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    key.len() >= 3
        && first.is_ascii_uppercase()
        && chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn parse_env_file(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let key = trimmed[..eq].trim().to_string();
            let val = trimmed[eq + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !key.is_empty() {
                map.insert(key, val);
            }
        }
    }
    map
}

fn serialize_env(vars: &HashMap<String, String>) -> String {
    let mut lines: Vec<String> = vars
        .iter()
        .map(|(k, v)| {
            if v.contains(' ') || v.contains('"') || v.contains('\'') {
                format!("{}=\"{}\"", k, v.replace('"', "\\\""))
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

fn modified_at(path: &Path) -> Option<String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
}

fn now_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn ensure_private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;

    Ok(())
}

fn write_private_env_file(path: &Path, vars: &HashMap<String, String>) -> Result<(), String> {
    if vars.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    let Some(parent) = path.parent() else {
        return Err("Invalid env file path".to_string());
    };
    ensure_private_dir(parent)?;
    fs::write(path, serialize_env(vars)).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;

    Ok(())
}

fn write_private_json_file(path: &Path, content: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err("Invalid index file path".to_string());
    };
    ensure_private_dir(parent)?;
    fs::write(path, content).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;

    Ok(())
}

fn native_storage_kind() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos_keychain"
    }
    #[cfg(target_os = "windows")]
    {
        "windows_credential_manager"
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        "secret_service"
    }
    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "linux",
        target_os = "macos",
        target_os = "windows",
    )))]
    {
        "secure_store"
    }
}

trait CredentialBackend: Send + Sync {
    fn storage_kind(&self) -> &'static str;
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, value: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
}

struct NativeCredentialBackend;

impl NativeCredentialBackend {
    fn init_store(&self) -> Result<(), String> {
        let config = HashMap::new();

        #[cfg(target_os = "macos")]
        {
            use apple_native_keyring_store::keychain::Store;
            keyring_core::set_default_store(
                Store::new_with_configuration(&config).map_err(|e| e.to_string())?,
            );
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            use windows_native_keyring_store::Store;
            keyring_core::set_default_store(
                Store::new_with_configuration(&config).map_err(|e| e.to_string())?,
            );
            return Ok(());
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            use dbus_secret_service_keyring_store::Store;
            keyring_core::set_default_store(
                Store::new_with_configuration(&config).map_err(|e| e.to_string())?,
            );
            return Ok(());
        }

        #[cfg(not(any(
            target_os = "freebsd",
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
        )))]
        {
            Err("No supported OS credential store is available on this platform".to_string())
        }
    }

    fn entry(&self, account: &str) -> Result<keyring_core::Entry, String> {
        self.init_store()?;
        keyring_core::Entry::new(CREDENTIAL_SERVICE, account).map_err(|e| e.to_string())
    }
}

impl CredentialBackend for NativeCredentialBackend {
    fn storage_kind(&self) -> &'static str {
        native_storage_kind()
    }

    fn get(&self, account: &str) -> Result<Option<String>, String> {
        let entry = self.entry(account)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        self.entry(account)?
            .set_password(value)
            .map_err(|e| e.to_string())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        let entry = self.entry(account)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
#[derive(Debug, Default)]
struct MemoryCredentialBackend {
    values: std::sync::Mutex<BTreeMap<String, String>>,
}

#[cfg(test)]
impl MemoryCredentialBackend {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn get_saved(&self, account: &str) -> Option<String> {
        self.values.lock().ok()?.get(account).cloned()
    }
}

#[cfg(test)]
impl CredentialBackend for MemoryCredentialBackend {
    fn storage_kind(&self) -> &'static str {
        "secure_store"
    }

    fn get(&self, account: &str) -> Result<Option<String>, String> {
        Ok(self
            .values
            .lock()
            .map_err(|e| e.to_string())?
            .get(account)
            .cloned())
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        self.values
            .lock()
            .map_err(|e| e.to_string())?
            .insert(account.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        self.values
            .lock()
            .map_err(|e| e.to_string())?
            .remove(account);
        Ok(())
    }
}

#[cfg(test)]
#[derive(Debug)]
struct FailingCredentialBackend {
    reason: String,
}

#[cfg(test)]
impl FailingCredentialBackend {
    fn new(reason: &str) -> Arc<Self> {
        Arc::new(Self {
            reason: reason.to_string(),
        })
    }
}

#[cfg(test)]
impl CredentialBackend for FailingCredentialBackend {
    fn storage_kind(&self) -> &'static str {
        "secure_store"
    }

    fn get(&self, _account: &str) -> Result<Option<String>, String> {
        Err(self.reason.clone())
    }

    fn set(&self, _account: &str, _value: &str) -> Result<(), String> {
        Err(self.reason.clone())
    }

    fn delete(&self, _account: &str) -> Result<(), String> {
        Err(self.reason.clone())
    }
}

fn default_configured() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnvIndexVariable {
    #[serde(default = "default_configured")]
    configured: bool,
    updated_at: Option<String>,
    storage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvIndex {
    version: u32,
    variables: BTreeMap<String, EnvIndexVariable>,
}

impl Default for EnvIndex {
    fn default() -> Self {
        Self {
            version: 1,
            variables: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgEnvVariable {
    pub name: String,
    pub configured: bool,
    pub updated_at: Option<String>,
    pub storage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyEnvVariableSummary {
    pub name: String,
    pub configured: bool,
    pub skills: Vec<String>,
    pub value_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyMigrationItem {
    pub name: String,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyMigrationConflict {
    pub name: String,
    pub skills: Vec<String>,
    pub value_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvMigrationSummary {
    pub migrated: Vec<LegacyMigrationItem>,
    pub migratable: Vec<LegacyMigrationItem>,
    pub conflicts: Vec<LegacyMigrationConflict>,
    pub legacy_variables: Vec<LegacyEnvVariableSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCleanupItem {
    pub name: String,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCleanupSkipped {
    pub name: String,
    pub skills: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCleanupSummary {
    pub cleaned: Vec<LegacyCleanupItem>,
    pub removed_files: Vec<String>,
    pub skipped: Vec<LegacyCleanupSkipped>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureStoreMigrationItem {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureStoreMigrationFailure {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureStoreMigrationSummary {
    pub migrated: Vec<SecureStoreMigrationItem>,
    pub failed: Vec<SecureStoreMigrationFailure>,
}

pub struct EnvStore {
    root: PathBuf,
    org: String,
    credential_backend: Arc<dyn CredentialBackend>,
}

impl EnvStore {
    pub fn new(org: &str) -> Self {
        Self::with_root(env_root(), org)
    }

    pub fn with_root(root: PathBuf, org: &str) -> Self {
        Self::with_credential_backend(root, org, Arc::new(NativeCredentialBackend))
    }

    fn with_credential_backend(
        root: PathBuf,
        org: &str,
        credential_backend: Arc<dyn CredentialBackend>,
    ) -> Self {
        Self {
            root,
            org: org.to_string(),
            credential_backend,
        }
    }

    fn org_dir(&self) -> PathBuf {
        self.root.join(&self.org)
    }

    fn org_variables_file(&self) -> PathBuf {
        self.org_dir().join(ORG_VARIABLES_FILE)
    }

    fn org_index_file(&self) -> PathBuf {
        self.org_dir().join(ORG_INDEX_FILE)
    }

    fn credential_account(&self, key: &str) -> String {
        format!("{}:env:{}", self.org, key)
    }

    fn read_index(&self) -> Result<EnvIndex, String> {
        let path = self.org_index_file();
        if !path.exists() {
            return Ok(EnvIndex::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str::<EnvIndex>(&content).map_err(|e| e.to_string())
    }

    fn write_index(&self, index: &EnvIndex) -> Result<(), String> {
        let content = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
        write_private_json_file(&self.org_index_file(), &content)
    }

    fn read_fallback_variables(&self) -> Result<HashMap<String, String>, String> {
        let path = self.org_variables_file();
        if !path.exists() {
            return Ok(HashMap::new());
        }

        Ok(
            parse_env_file(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
                .into_iter()
                .filter_map(|(key, value)| {
                    let name = normalize_key(&key);
                    is_valid_env_key(&name).then_some((name, value))
                })
                .collect(),
        )
    }

    fn write_fallback_variables(&self, vars: &HashMap<String, String>) -> Result<(), String> {
        write_private_env_file(&self.org_variables_file(), vars)
    }

    fn mark_index_variable(&self, key: &str, storage: &str) -> Result<(), String> {
        let mut index = self.read_index()?;
        index.variables.insert(
            key.to_string(),
            EnvIndexVariable {
                configured: true,
                updated_at: Some(now_timestamp()),
                storage: storage.to_string(),
            },
        );
        self.write_index(&index)
    }

    fn remove_index_variable(&self, key: &str) -> Result<(), String> {
        let mut index = self.read_index()?;
        index.variables.remove(key);
        self.write_index(&index)
    }

    #[cfg(test)]
    pub fn list_variables(&self) -> Result<HashMap<String, String>, String> {
        let mut result = HashMap::new();
        let fallback = self.read_fallback_variables()?;
        let index = self.read_index()?;

        for (name, metadata) in index.variables {
            if !metadata.configured || metadata.storage == FALLBACK_FILE_STORAGE {
                continue;
            }
            if let Some(value) = self
                .credential_backend
                .get(&self.credential_account(&name))?
                .filter(|value| !value.trim().is_empty())
            {
                result.insert(name, value);
            }
        }

        for (name, value) in fallback {
            result.entry(name).or_insert(value);
        }

        Ok(result)
    }

    pub fn list_variable_summaries(&self) -> Result<Vec<OrgEnvVariable>, String> {
        let mut by_name = BTreeMap::<String, OrgEnvVariable>::new();

        for (name, metadata) in self.read_index()?.variables {
            if !metadata.configured {
                continue;
            }
            by_name.insert(
                name.clone(),
                OrgEnvVariable {
                    name,
                    configured: true,
                    updated_at: metadata.updated_at,
                    storage: metadata.storage,
                },
            );
        }

        let fallback_updated_at = modified_at(&self.org_variables_file());
        for (name, value) in self.read_fallback_variables()? {
            if value.trim().is_empty() || by_name.contains_key(&name) {
                continue;
            }
            by_name.insert(
                name.clone(),
                OrgEnvVariable {
                    name,
                    configured: true,
                    updated_at: fallback_updated_at.clone(),
                    storage: FALLBACK_FILE_STORAGE.to_string(),
                },
            );
        }

        let mut summaries: Vec<OrgEnvVariable> = by_name.into_values().collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(summaries)
    }

    pub fn get_variable(&self, key: &str) -> Result<Option<String>, String> {
        let name = normalize_key(key);
        if !is_valid_env_key(&name) {
            return Err(format!("Invalid environment variable name: {}", key));
        }
        let index = self.read_index()?;
        if let Some(metadata) = index.variables.get(&name) {
            if metadata.configured && metadata.storage != FALLBACK_FILE_STORAGE {
                let account = self.credential_account(&name);
                return self.credential_backend.get(&account);
            }
        }

        Ok(self.read_fallback_variables()?.remove(&name))
    }

    pub fn is_variable_configured(&self, key: &str) -> Result<bool, String> {
        Ok(self
            .get_variable(key)?
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false))
    }

    pub fn set_variable(&self, key: &str, value: &str) -> Result<(), String> {
        let name = normalize_key(key);
        if !is_valid_env_key(&name) {
            return Err(format!("Invalid environment variable name: {}", key));
        }

        let account = self.credential_account(&name);
        match self.credential_backend.set(&account, value) {
            Ok(()) => {
                let mut fallback = self.read_fallback_variables()?;
                fallback.remove(&name);
                self.write_fallback_variables(&fallback)?;
                self.mark_index_variable(&name, self.credential_backend.storage_kind())
            }
            Err(_) => {
                let mut fallback = self.read_fallback_variables()?;
                fallback.insert(name.clone(), value.to_string());
                self.write_fallback_variables(&fallback)?;
                self.mark_index_variable(&name, FALLBACK_FILE_STORAGE)
            }
        }
    }

    pub fn delete_variable(&self, key: &str) -> Result<(), String> {
        let name = normalize_key(key);
        if !is_valid_env_key(&name) {
            return Err(format!("Invalid environment variable name: {}", key));
        }

        let _ = self
            .credential_backend
            .delete(&self.credential_account(&name));
        let mut fallback = self.read_fallback_variables()?;
        fallback.remove(&name);
        self.write_fallback_variables(&fallback)?;
        self.remove_index_variable(&name)
    }

    pub fn list_legacy_env_vars(&self) -> Result<Vec<SkillEnvVars>, String> {
        let dir = self.org_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().into_owned();
            if !fname.ends_with(".env") || fname == ORG_VARIABLES_FILE {
                continue;
            }
            let skill = fname.trim_end_matches(".env").to_string();
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            let vars = parse_env_file(&content);
            if !vars.is_empty() {
                results.push(SkillEnvVars { skill, vars });
            }
        }

        results.sort_by(|a, b| a.skill.cmp(&b.skill));
        Ok(results)
    }

    pub fn preview_legacy_migration(&self) -> Result<EnvMigrationSummary, String> {
        self.build_legacy_migration(false)
    }

    pub fn migrate_legacy_variables(&self) -> Result<EnvMigrationSummary, String> {
        self.build_legacy_migration(true)
    }

    pub fn cleanup_legacy_variables(&self) -> Result<LegacyCleanupSummary, String> {
        let dir = self.org_dir();
        if !dir.exists() {
            return Ok(LegacyCleanupSummary {
                cleaned: Vec::new(),
                removed_files: Vec::new(),
                skipped: Vec::new(),
            });
        }

        let mut cleaned_by_name: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut skipped_by_reason: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        let mut removed_files = Vec::new();

        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
            let fname = entry.file_name().to_string_lossy().into_owned();
            if !fname.ends_with(".env") || fname == ORG_VARIABLES_FILE {
                continue;
            }

            let skill = fname.trim_end_matches(".env").to_string();
            let path = entry.path();
            let vars = parse_env_file(&fs::read_to_string(&path).unwrap_or_default());
            if vars.is_empty() {
                continue;
            }

            let mut remaining = vars.clone();
            let mut changed = false;

            for (raw_name, value) in vars {
                let name = normalize_key(&raw_name);
                if !is_valid_env_key(&name) || value.trim().is_empty() {
                    continue;
                }

                match self.get_variable(&name)? {
                    Some(current) if current == value => {
                        remaining.remove(&raw_name);
                        cleaned_by_name
                            .entry(name)
                            .or_default()
                            .insert(skill.clone());
                        changed = true;
                    }
                    Some(_) => {
                        skipped_by_reason
                            .entry((name, "valueMismatch".to_string()))
                            .or_default()
                            .insert(skill.clone());
                    }
                    None => {
                        skipped_by_reason
                            .entry((name, "notConfigured".to_string()))
                            .or_default()
                            .insert(skill.clone());
                    }
                }
            }

            if changed {
                write_private_env_file(&path, &remaining)?;
                if !path.exists() {
                    removed_files.push(skill);
                }
            }
        }

        let cleaned = cleaned_by_name
            .into_iter()
            .map(|(name, skills)| LegacyCleanupItem {
                name,
                skills: skills.into_iter().collect(),
            })
            .collect();
        let skipped = skipped_by_reason
            .into_iter()
            .map(|((name, reason), skills)| LegacyCleanupSkipped {
                name,
                reason,
                skills: skills.into_iter().collect(),
            })
            .collect();
        removed_files.sort();

        Ok(LegacyCleanupSummary {
            cleaned,
            removed_files,
            skipped,
        })
    }

    pub fn migrate_fallback_file_to_secure_store(
        &self,
    ) -> Result<SecureStoreMigrationSummary, String> {
        let fallback = self.read_fallback_variables()?;
        let mut remaining = fallback.clone();
        let mut migrated = Vec::new();
        let mut failed = Vec::new();

        for (name, value) in fallback {
            if value.trim().is_empty() {
                remaining.remove(&name);
                continue;
            }

            let account = self.credential_account(&name);
            match self.credential_backend.set(&account, &value) {
                Ok(()) => {
                    remaining.remove(&name);
                    self.mark_index_variable(&name, self.credential_backend.storage_kind())?;
                    migrated.push(SecureStoreMigrationItem { name });
                }
                Err(reason) => {
                    failed.push(SecureStoreMigrationFailure { name, reason });
                }
            }
        }

        self.write_fallback_variables(&remaining)?;
        migrated.sort_by(|a, b| a.name.cmp(&b.name));
        failed.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(SecureStoreMigrationSummary { migrated, failed })
    }

    fn build_legacy_migration(&self, apply: bool) -> Result<EnvMigrationSummary, String> {
        let org_vars: BTreeSet<String> = self
            .list_variable_summaries()?
            .into_iter()
            .map(|variable| variable.name)
            .collect();
        let legacy = self.list_legacy_env_vars()?;
        let mut values_by_name: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> =
            BTreeMap::new();

        for skill_env in legacy {
            for (raw_name, value) in skill_env.vars {
                if value.trim().is_empty() {
                    continue;
                }
                let name = normalize_key(&raw_name);
                if !is_valid_env_key(&name) {
                    continue;
                }
                values_by_name
                    .entry(name)
                    .or_default()
                    .entry(value)
                    .or_default()
                    .insert(skill_env.skill.clone());
            }
        }

        let mut migrated = Vec::new();
        let mut migratable = Vec::new();
        let mut conflicts = Vec::new();
        let mut legacy_variables = Vec::new();

        for (name, values) in values_by_name {
            let skills: Vec<String> = values
                .values()
                .flat_map(|skill_names| skill_names.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let value_count = values.len();
            let status = if value_count > 1 {
                "conflict"
            } else if org_vars.contains(&name) {
                "alreadyConfigured"
            } else {
                "migratable"
            };

            legacy_variables.push(LegacyEnvVariableSummary {
                name: name.clone(),
                configured: true,
                skills: skills.clone(),
                value_count,
                status: status.to_string(),
            });

            if status == "conflict" {
                conflicts.push(LegacyMigrationConflict {
                    name,
                    skills,
                    value_count,
                });
                continue;
            }

            if status == "migratable" {
                let item = LegacyMigrationItem {
                    name: name.clone(),
                    skills,
                };

                if apply {
                    if let Some(value) = values.keys().next() {
                        self.set_variable(&name, value)?;
                        migrated.push(item);
                    }
                } else {
                    migratable.push(item);
                }
            }
        }

        Ok(EnvMigrationSummary {
            migrated,
            migratable,
            conflicts,
            legacy_variables,
        })
    }
}

#[tauri::command]
pub fn get_org_env_var(org: String, key: String) -> Result<Option<String>, String> {
    EnvStore::new(&org).get_variable(&key)
}

#[tauri::command]
pub fn set_org_env_var(org: String, key: String, value: String) -> Result<(), String> {
    EnvStore::new(&org).set_variable(&key, value.trim())
}

#[tauri::command]
pub fn delete_org_env_var(org: String, key: String) -> Result<(), String> {
    EnvStore::new(&org).delete_variable(&key)
}

#[tauri::command]
pub fn list_org_env_vars(org: String) -> Result<Vec<OrgEnvVariable>, String> {
    EnvStore::new(&org).list_variable_summaries()
}

#[tauri::command]
pub fn preview_legacy_env_migration(org: String) -> Result<EnvMigrationSummary, String> {
    EnvStore::new(&org).preview_legacy_migration()
}

#[tauri::command]
pub fn migrate_legacy_env_vars(org: String) -> Result<EnvMigrationSummary, String> {
    EnvStore::new(&org).migrate_legacy_variables()
}

#[tauri::command]
pub fn cleanup_legacy_env_vars(org: String) -> Result<LegacyCleanupSummary, String> {
    EnvStore::new(&org).cleanup_legacy_variables()
}

#[tauri::command]
pub fn migrate_org_env_file_to_secure_store(
    org: String,
) -> Result<SecureStoreMigrationSummary, String> {
    EnvStore::new(&org).migrate_fallback_file_to_secure_store()
}

#[tauri::command]
pub fn get_env_vars(org: String, skill: String) -> Result<HashMap<String, String>, String> {
    let path = env_file(&org, &skill);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(parse_env_file(&content))
}

#[tauri::command]
pub fn set_env_vars(
    org: String,
    skill: String,
    vars: HashMap<String, String>,
) -> Result<(), String> {
    let dir = env_dir(&org);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let path = env_file(&org, &skill);

    // Merge with existing
    let mut existing = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        parse_env_file(&content)
    } else {
        HashMap::new()
    };

    for (k, v) in vars {
        existing.insert(k, v);
    }

    let content = serialize_env(&existing);
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_env_vars(org: String, skill: String, keys: Vec<String>) -> Result<(), String> {
    let path = env_file(&org, &skill);
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut vars = parse_env_file(&content);

    for key in &keys {
        vars.remove(key);
    }

    if vars.is_empty() {
        fs::remove_file(&path).map_err(|e| e.to_string())
    } else {
        let content = serialize_env(&vars);
        fs::write(&path, content).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillEnvVars {
    pub skill: String,
    pub vars: HashMap<String, String>,
}

#[tauri::command]
pub fn list_all_env_vars(org: String) -> Result<Vec<SkillEnvVars>, String> {
    EnvStore::new(&org).list_legacy_env_vars()
}

#[tauri::command]
pub fn import_env_file(
    org: String,
    skill: String,
    file_path: String,
) -> Result<HashMap<String, String>, String> {
    let content = fs::read_to_string(&file_path).map_err(|e| format!("Cannot read file: {}", e))?;
    let vars = parse_env_file(&content);

    if !vars.is_empty() {
        set_env_vars(org, skill, vars.clone())?;
    }

    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::{EnvStore, FailingCredentialBackend, MemoryCredentialBackend};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skillreg-env-store-{}-{}-{}",
            label,
            std::process::id(),
            now
        ));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_legacy(root: &Path, org: &str, skill: &str, content: &str) {
        let dir = root.join(org);
        fs::create_dir_all(&dir).expect("create org dir");
        fs::write(dir.join(format!("{}.env", skill)), content).expect("write legacy file");
    }

    #[test]
    fn stores_lists_gets_and_deletes_org_variables_in_secure_store() {
        let root = temp_root("crud");
        let backend = MemoryCredentialBackend::new();
        let store = EnvStore::with_credential_backend(root.clone(), "kairia", backend.clone());

        store
            .set_variable("OPENAI_API_KEY", "placeholder-value")
            .expect("set variable");

        assert_eq!(
            store
                .get_variable("OPENAI_API_KEY")
                .expect("get variable")
                .as_deref(),
            Some("placeholder-value")
        );
        assert_eq!(
            store
                .list_variables()
                .expect("list variable values")
                .get("OPENAI_API_KEY")
                .map(String::as_str),
            Some("placeholder-value")
        );
        assert_eq!(
            store
                .list_variable_summaries()
                .expect("list variables")
                .iter()
                .map(|item| (item.name.as_str(), item.storage.as_str()))
                .collect::<Vec<_>>(),
            vec![("OPENAI_API_KEY", "secure_store")]
        );
        assert_eq!(
            backend.get_saved("kairia:env:OPENAI_API_KEY").as_deref(),
            Some("placeholder-value")
        );
        assert!(
            !root.join("kairia").join("variables.env").exists(),
            "secure store writes must not create plaintext variables.env"
        );
        let index = fs::read_to_string(root.join("kairia").join("index.json")).expect("read index");
        assert!(index.contains("OPENAI_API_KEY"));
        assert!(!index.contains("placeholder-value"));

        store
            .delete_variable("OPENAI_API_KEY")
            .expect("delete variable");

        assert_eq!(
            store
                .get_variable("OPENAI_API_KEY")
                .expect("get deleted variable"),
            None
        );
        assert_eq!(backend.get_saved("kairia:env:OPENAI_API_KEY"), None);
    }

    #[test]
    fn falls_back_to_private_file_when_secure_store_is_unavailable() {
        let root = temp_root("fallback");
        let store = EnvStore::with_credential_backend(
            root.clone(),
            "kairia",
            FailingCredentialBackend::new("locked keychain"),
        );

        store
            .set_variable("OPENAI_API_KEY", "fallback-placeholder")
            .expect("set fallback variable");

        assert_eq!(
            store
                .get_variable("OPENAI_API_KEY")
                .expect("get fallback variable")
                .as_deref(),
            Some("fallback-placeholder")
        );
        assert_eq!(
            store
                .list_variable_summaries()
                .expect("list fallback variables")
                .iter()
                .map(|item| (item.name.as_str(), item.storage.as_str()))
                .collect::<Vec<_>>(),
            vec![("OPENAI_API_KEY", "fallback_file")]
        );
        assert!(
            root.join("kairia").join("variables.env").exists(),
            "fallback storage should use variables.env"
        );
    }

    #[test]
    fn reads_existing_index_entries_that_do_not_include_configured_flag() {
        let root = temp_root("legacy-index");
        fs::create_dir_all(root.join("kairia")).expect("create org dir");
        fs::write(
            root.join("kairia").join("index.json"),
            r#"{
  "version": 1,
  "variables": {
    "OPENAI_API_KEY": {
      "updatedAt": "2026-05-20T18:42:59.076Z",
      "storage": "fallback_file"
    }
  }
}"#,
        )
        .expect("write legacy index");
        fs::write(
            root.join("kairia").join("variables.env"),
            "OPENAI_API_KEY=legacy-placeholder\n",
        )
        .expect("write fallback env file");
        let store = EnvStore::with_root(root, "kairia");

        let summaries = store
            .list_variable_summaries()
            .expect("legacy index should remain readable");

        assert_eq!(
            summaries
                .iter()
                .map(|item| (item.name.as_str(), item.configured, item.storage.as_str()))
                .collect::<Vec<_>>(),
            vec![("OPENAI_API_KEY", true, "fallback_file")]
        );
    }

    #[test]
    fn migrates_fallback_file_values_to_secure_store_on_explicit_request() {
        let root = temp_root("secure-migration");
        fs::create_dir_all(root.join("kairia")).expect("create org dir");
        fs::write(
            root.join("kairia").join("variables.env"),
            "OPENAI_API_KEY=phase-two-placeholder\n",
        )
        .expect("write phase 2 store");
        let backend = MemoryCredentialBackend::new();
        let store = EnvStore::with_credential_backend(root.clone(), "kairia", backend.clone());

        let summary = store
            .migrate_fallback_file_to_secure_store()
            .expect("migrate fallback file");

        assert_eq!(
            summary
                .migrated
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["OPENAI_API_KEY"]
        );
        assert_eq!(
            backend.get_saved("kairia:env:OPENAI_API_KEY").as_deref(),
            Some("phase-two-placeholder")
        );
        assert!(
            !root.join("kairia").join("variables.env").exists(),
            "explicit secure migration should remove migrated plaintext values"
        );
    }

    #[test]
    fn migrates_identical_legacy_values_to_org_storage_without_deleting_legacy_files() {
        let root = temp_root("migrate");
        write_legacy(
            &root,
            "kairia",
            "reviewer",
            "OPENAI_API_KEY=shared-placeholder\n",
        );
        write_legacy(
            &root,
            "kairia",
            "writer",
            "OPENAI_API_KEY=shared-placeholder\n",
        );
        let backend = MemoryCredentialBackend::new();
        let store = EnvStore::with_credential_backend(root.clone(), "kairia", backend.clone());

        let summary = store.migrate_legacy_variables().expect("migrate legacy");

        assert_eq!(
            summary
                .migrated
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["OPENAI_API_KEY"]
        );
        assert!(summary.conflicts.is_empty());
        assert_eq!(
            store
                .get_variable("OPENAI_API_KEY")
                .expect("read migrated value")
                .as_deref(),
            Some("shared-placeholder")
        );
        assert_eq!(
            backend.get_saved("kairia:env:OPENAI_API_KEY").as_deref(),
            Some("shared-placeholder")
        );
        assert!(root.join("kairia").join("reviewer.env").exists());
        assert!(root.join("kairia").join("writer.env").exists());
    }

    #[test]
    fn cleans_up_legacy_values_that_match_org_storage() {
        let root = temp_root("cleanup");
        write_legacy(
            &root,
            "kairia",
            "reviewer",
            "OPENAI_API_KEY=shared-placeholder\n",
        );
        write_legacy(
            &root,
            "kairia",
            "writer",
            "OPENAI_API_KEY=shared-placeholder\n",
        );
        let backend = MemoryCredentialBackend::new();
        let store = EnvStore::with_credential_backend(root.clone(), "kairia", backend);

        store.migrate_legacy_variables().expect("migrate legacy");
        let summary = store.cleanup_legacy_variables().expect("cleanup legacy");

        assert_eq!(summary.cleaned.len(), 1);
        assert_eq!(summary.cleaned[0].name, "OPENAI_API_KEY");
        assert_eq!(summary.cleaned[0].skills, vec!["reviewer", "writer"]);
        assert_eq!(summary.removed_files, vec!["reviewer", "writer"]);
        assert!(store
            .list_legacy_env_vars()
            .expect("list legacy after cleanup")
            .is_empty());
    }

    #[test]
    fn keeps_legacy_values_that_do_not_match_org_storage() {
        let root = temp_root("cleanup-mismatch");
        write_legacy(
            &root,
            "kairia",
            "reviewer",
            "GITHUB_TOKEN=old-placeholder\n",
        );
        let backend = MemoryCredentialBackend::new();
        let store = EnvStore::with_credential_backend(root.clone(), "kairia", backend);

        store
            .set_variable("GITHUB_TOKEN", "canonical-placeholder")
            .expect("set canonical value");
        let summary = store.cleanup_legacy_variables().expect("cleanup legacy");

        assert!(summary.cleaned.is_empty());
        assert!(summary.removed_files.is_empty());
        assert_eq!(summary.skipped.len(), 1);
        assert_eq!(summary.skipped[0].name, "GITHUB_TOKEN");
        assert_eq!(
            fs::read_to_string(root.join("kairia").join("reviewer.env"))
                .expect("legacy file should remain"),
            "GITHUB_TOKEN=old-placeholder\n"
        );
    }

    #[test]
    fn reports_legacy_conflicts_without_overwriting_org_storage() {
        let root = temp_root("conflict");
        write_legacy(&root, "kairia", "reviewer", "GITHUB_TOKEN=value-one\n");
        write_legacy(&root, "kairia", "release-notes", "GITHUB_TOKEN=value-two\n");
        let store = EnvStore::with_root(root, "kairia");

        let summary = store.migrate_legacy_variables().expect("migrate legacy");

        assert!(summary.migrated.is_empty());
        assert_eq!(summary.conflicts.len(), 1);
        assert_eq!(summary.conflicts[0].name, "GITHUB_TOKEN");
        assert_eq!(summary.conflicts[0].value_count, 2);
        assert_eq!(
            store
                .get_variable("GITHUB_TOKEN")
                .expect("conflicting value should not be migrated"),
            None
        );
    }
}
