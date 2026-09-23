use crate::managed_skills::errors::{ManagedError, ManagedErrorCode};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub max_files: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_depth: usize,
    pub max_path_bytes: usize,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_files: 10_000,
            max_file_bytes: 50 * 1024 * 1024,
            max_total_bytes: 250 * 1024 * 1024,
            max_depth: 32,
            max_path_bytes: 4_096,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveExtraction {
    pub file_count: usize,
    pub total_bytes: u64,
}

#[derive(Debug)]
struct ArchiveLayout {
    strip_root: Option<OsString>,
    file_count: usize,
    total_bytes: u64,
}

pub fn validate_and_extract_archive(
    archive_path: &Path,
    staging: &Path,
) -> Result<ArchiveExtraction, ManagedError> {
    validate_and_extract_archive_with_limits(archive_path, staging, ArchiveLimits::default())
}

pub fn validate_and_extract_archive_with_limits(
    archive_path: &Path,
    staging: &Path,
    limits: ArchiveLimits,
) -> Result<ArchiveExtraction, ManagedError> {
    reject_existing_or_linked_staging(staging)?;
    let layout = inspect_archive(archive_path, limits)?;
    let parent = staging
        .parent()
        .ok_or_else(|| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }
    fs::create_dir(staging).map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;

    let extraction_result = extract_archive(archive_path, staging, &layout, limits);
    if extraction_result.is_err() {
        cleanup_staging(staging);
    }
    extraction_result?;
    Ok(ArchiveExtraction {
        file_count: layout.file_count,
        total_bytes: layout.total_bytes,
    })
}

pub fn is_safe_archive_path(path: &Path, limits: ArchiveLimits) -> bool {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.as_os_str().to_string_lossy().contains(['\\', ':'])
        || path.as_os_str().len() > limits.max_path_bytes
    {
        return false;
    }
    let components = path.components().collect::<Vec<_>>();
    !components.is_empty()
        && components.len() <= limits.max_depth
        && components
            .iter()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub fn compute_tree_hash(root: &Path) -> Result<String, ManagedError> {
    let root_metadata = fs::symlink_metadata(root)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }

    let mut files = Vec::new();
    collect_regular_files(root, root, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    for relative in files {
        let relative_bytes = relative.to_string_lossy().as_bytes().to_vec();
        hasher.update((relative_bytes.len() as u64).to_le_bytes());
        hasher.update(&relative_bytes);
        let mut file = File::open(root.join(&relative))
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn inspect_archive(
    archive_path: &Path,
    limits: ArchiveLimits,
) -> Result<ArchiveLayout, ManagedError> {
    let file =
        File::open(archive_path).map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let entries = archive
        .entries()
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    let mut file_paths = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;

    for entry in entries {
        let entry = entry.map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        let path = entry
            .path()
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?
            .into_owned();
        if !is_safe_archive_path(&path, limits) {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        if entry_type.is_file() {
            let size = entry.size();
            file_count = file_count
                .checked_add(1)
                .ok_or_else(|| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
            total_bytes = total_bytes
                .checked_add(size)
                .ok_or_else(|| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
            if file_count > limits.max_files
                || size > limits.max_file_bytes
                || total_bytes > limits.max_total_bytes
            {
                return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
            }
            file_paths.push(path.clone());
        }
        let normalized = path.to_string_lossy().to_ascii_lowercase();
        if !seen_paths.insert(normalized) {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
    }
    if file_count == 0 {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }

    let strip_root = common_file_root(&file_paths);
    let mut destinations = HashSet::new();
    for path in &file_paths {
        let destination = strip_archive_root(path, strip_root.as_deref())?;
        let normalized = destination.to_string_lossy().to_ascii_lowercase();
        if destination.as_os_str().is_empty() || !destinations.insert(normalized) {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
    }

    Ok(ArchiveLayout {
        strip_root,
        file_count,
        total_bytes,
    })
}

fn extract_archive(
    archive_path: &Path,
    staging: &Path,
    layout: &ArchiveLayout,
    limits: ArchiveLimits,
) -> Result<(), ManagedError> {
    let file =
        File::open(archive_path).map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let entries = archive
        .entries()
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;

    for entry in entries {
        let mut entry = entry.map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        let source = entry
            .path()
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?
            .into_owned();
        if !is_safe_archive_path(&source, limits) {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        let relative = strip_archive_root(&source, layout.strip_root.as_deref())?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let destination = staging.join(&relative);
        if !destination.starts_with(staging) {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }

        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            create_safe_subdirectories(staging, &relative)?;
            continue;
        }
        if !entry_type.is_file() {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
        create_safe_subdirectories(staging, parent_relative)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        let expected_size = entry.size();
        let written = std::io::copy(&mut entry, &mut output)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        if written != expected_size {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        output
            .flush()
            .and_then(|_| output.sync_all())
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        #[cfg(unix)]
        {
            let archived_mode = entry.header().mode().unwrap_or(0o644);
            let mode = if archived_mode & 0o111 == 0 {
                0o600
            } else {
                0o700
            };
            fs::set_permissions(&destination, fs::Permissions::from_mode(mode))
                .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        }
    }
    Ok(())
}

fn common_file_root(paths: &[PathBuf]) -> Option<OsString> {
    let mut root: Option<OsString> = None;
    for path in paths {
        let mut components = path.components();
        let first = match components.next() {
            Some(Component::Normal(component)) => component.to_os_string(),
            _ => return None,
        };
        if components.next().is_none() {
            return None;
        }
        match &root {
            Some(existing) if existing != &first => return None,
            None => root = Some(first),
            _ => {}
        }
    }
    root
}

fn strip_archive_root(
    path: &Path,
    root: Option<&std::ffi::OsStr>,
) -> Result<PathBuf, ManagedError> {
    let relative = if let Some(root) = root {
        path.strip_prefix(Path::new(root))
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?
    } else {
        path
    };
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }
    Ok(relative.to_path_buf())
}

fn create_safe_subdirectories(root: &Path, relative: &Path) -> Result<(), ManagedError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
            }
            Err(_) => return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe)),
        }
    }
    Ok(())
}

fn collect_regular_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), ManagedError> {
    let entries =
        fs::read_dir(current).map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    for entry in entries {
        let entry = entry.map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        if metadata.file_type().is_symlink() {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        if metadata.is_dir() {
            collect_regular_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(
                path.strip_prefix(root)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?
                    .to_path_buf(),
            );
        } else {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
    }
    Ok(())
}

fn reject_existing_or_linked_staging(staging: &Path) -> Result<(), ManagedError> {
    match fs::symlink_metadata(staging) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) | Err(_) => Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe)),
    }
}

fn cleanup_staging(staging: &Path) {
    if let Ok(metadata) = fs::symlink_metadata(staging) {
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            let _ = fs::remove_dir_all(staging);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_skills::errors::ManagedErrorCode;
    use flate2::{write::GzEncoder, Compression};
    use std::{fs, io::Cursor, path::PathBuf};
    use tar::{Builder, EntryType, Header};
    use uuid::Uuid;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "skillreg-managed-archive-{name}-{}",
                Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn archive(entries: &[(&str, EntryType, &[u8])]) -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = Builder::new(encoder);
        for (path, entry_type, payload) in entries {
            let mut header = Header::new_gnu();
            header.set_entry_type(*entry_type);
            header.set_mode(0o644);
            header.set_size(payload.len() as u64);
            set_raw_path(&mut header, path);
            if entry_type.is_symlink() || entry_type.is_hard_link() {
                header.set_link_name("outside").unwrap();
            }
            header.set_cksum();
            builder
                .append(&header, Cursor::new(*payload))
                .expect("test archive should be writable");
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn set_raw_path(header: &mut Header, path: &str) {
        let bytes = path.as_bytes();
        assert!(bytes.len() < 100);
        header.as_mut_bytes()[..100].fill(0);
        header.as_mut_bytes()[..bytes.len()].copy_from_slice(bytes);
    }

    fn write_archive(home: &TestDir, bytes: &[u8]) -> PathBuf {
        let path = home.path.join("skill.tar.gz");
        fs::write(&path, bytes).unwrap();
        path
    }

    fn small_limits() -> ArchiveLimits {
        ArchiveLimits {
            max_files: 10,
            max_file_bytes: 64,
            max_total_bytes: 256,
            max_depth: 4,
            max_path_bytes: 128,
        }
    }

    #[test]
    fn valid_rooted_archive_is_flattened_into_staging() {
        let home = TestDir::new("valid");
        let bytes = archive(&[
            (
                "review-helper/SKILL.md",
                EntryType::Regular,
                b"---\nname: review-helper\ndescription: Review\n---\n",
            ),
            (
                "review-helper/scripts/run.sh",
                EntryType::Regular,
                b"echo ok",
            ),
        ]);
        let archive_path = write_archive(&home, &bytes);
        let staging = home.path.join("staging");

        let extracted =
            validate_and_extract_archive_with_limits(&archive_path, &staging, small_limits())
                .unwrap();

        assert_eq!(extracted.file_count, 2);
        assert!(staging.join("SKILL.md").is_file());
        assert!(staging.join("scripts/run.sh").is_file());
        assert_eq!(extracted.total_bytes, 55);
        assert_eq!(compute_tree_hash(&staging).unwrap().len(), 64);
    }

    #[test]
    fn traversal_absolute_and_windows_paths_are_rejected() {
        let home = TestDir::new("unsafe-paths");
        for (index, path) in ["../escape", "/absolute", r"C:\escape", r"..\escape"]
            .into_iter()
            .enumerate()
        {
            let bytes = archive(&[(path, EntryType::Regular, b"unsafe")]);
            let archive_path = home.path.join(format!("{index}.tar.gz"));
            fs::write(&archive_path, bytes).unwrap();
            let staging = home.path.join(format!("staging-{index}"));

            let error =
                validate_and_extract_archive_with_limits(&archive_path, &staging, small_limits())
                    .unwrap_err();

            assert_eq!(error.code(), ManagedErrorCode::ArchiveUnsafe);
            assert!(!staging.exists());
        }
    }

    #[test]
    fn symlinks_and_hardlinks_are_rejected() {
        let home = TestDir::new("links");
        for (index, entry_type) in [EntryType::Symlink, EntryType::Link]
            .into_iter()
            .enumerate()
        {
            let bytes = archive(&[("review-helper/link", entry_type, b"")]);
            let archive_path = home.path.join(format!("{index}.tar.gz"));
            fs::write(&archive_path, bytes).unwrap();
            let staging = home.path.join(format!("staging-{index}"));

            let error =
                validate_and_extract_archive_with_limits(&archive_path, &staging, small_limits())
                    .unwrap_err();

            assert_eq!(error.code(), ManagedErrorCode::ArchiveUnsafe);
            assert!(!staging.exists());
        }
    }

    #[test]
    fn excessive_depth_and_file_size_are_rejected() {
        let home = TestDir::new("limits");
        let cases = [
            archive(&[("root/a/b/c/d/file.txt", EntryType::Regular, b"deep")]),
            archive(&[(
                "root/large.txt",
                EntryType::Regular,
                b"this payload is larger than the test limit",
            )]),
        ];
        let mut limits = small_limits();
        limits.max_depth = 3;
        limits.max_file_bytes = 8;

        for (index, bytes) in cases.into_iter().enumerate() {
            let archive_path = home.path.join(format!("{index}.tar.gz"));
            fs::write(&archive_path, bytes).unwrap();
            let staging = home.path.join(format!("staging-{index}"));

            let error = validate_and_extract_archive_with_limits(&archive_path, &staging, limits)
                .unwrap_err();

            assert_eq!(error.code(), ManagedErrorCode::ArchiveUnsafe);
            assert!(!staging.exists());
        }
    }

    #[cfg(unix)]
    #[test]
    fn existing_symlinked_staging_directory_is_rejected_without_following_it() {
        let home = TestDir::new("staging-link");
        let outside = home.path.join("outside");
        fs::create_dir_all(&outside).unwrap();
        let staging = home.path.join("staging");
        std::os::unix::fs::symlink(&outside, &staging).unwrap();
        let archive_path = write_archive(
            &home,
            &archive(&[("root/SKILL.md", EntryType::Regular, b"safe")]),
        );

        let error =
            validate_and_extract_archive_with_limits(&archive_path, &staging, small_limits())
                .unwrap_err();

        assert_eq!(error.code(), ManagedErrorCode::ArchiveUnsafe);
        assert!(fs::read_dir(outside).unwrap().next().is_none());
    }
}
