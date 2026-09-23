use crate::managed_skills::{agents::Platform, errors::ManagedErrorCode, LinkKind};
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkInspection {
    Missing,
    Link {
        kind: LinkKind,
        target: PathBuf,
        target_exists: bool,
    },
    Other,
}

pub trait PlatformLinker: Send + Sync {
    fn platform(&self) -> Platform;
    fn link_kind(&self) -> LinkKind;
    fn validate_paths(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode>;
    fn inspect(&self, link: &Path) -> Result<LinkInspection, ManagedErrorCode>;
    fn create_dir_link(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode>;
    fn rename_link(&self, source: &Path, destination: &Path) -> Result<(), ManagedErrorCode>;
    fn remove_link(&self, link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode>;
}

#[derive(Debug, Clone, Copy)]
pub struct SystemPlatformLinker {
    platform: Platform,
}

impl SystemPlatformLinker {
    pub const fn current() -> Self {
        Self {
            platform: Platform::current(),
        }
    }
}

impl PlatformLinker for SystemPlatformLinker {
    fn platform(&self) -> Platform {
        self.platform
    }

    fn link_kind(&self) -> LinkKind {
        expected_link_kind(self.platform).unwrap_or(LinkKind::Symlink)
    }

    fn validate_paths(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        if !target.is_absolute()
            || !link.is_absolute()
            || has_unsafe_components(target)
            || has_unsafe_components(link)
        {
            return Err(ManagedErrorCode::ManagedPathOutsideRoot);
        }
        expected_link_kind(self.platform)?;

        #[cfg(target_os = "windows")]
        if self.platform == Platform::Windows {
            validate_windows_junction_paths(target, link)?;
        }

        Ok(())
    }

    fn inspect(&self, link: &Path) -> Result<LinkInspection, ManagedErrorCode> {
        inspect_link(link)
    }

    fn create_dir_link(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        self.validate_paths(target, link)?;

        #[cfg(unix)]
        {
            if self.platform != Platform::Macos && self.platform != Platform::Linux {
                return Err(ManagedErrorCode::BindingUnsupported);
            }
            std::os::unix::fs::symlink(target, link)
                .map_err(|_| ManagedErrorCode::BindingCreateFailed)
        }

        #[cfg(target_os = "windows")]
        {
            if self.platform != Platform::Windows {
                return Err(ManagedErrorCode::BindingUnsupported);
            }
            junction::create(target, link).map_err(|_| ManagedErrorCode::BindingCreateFailed)
        }

        #[cfg(not(any(unix, target_os = "windows")))]
        {
            let _ = (target, link);
            Err(ManagedErrorCode::BindingUnsupported)
        }
    }

    fn rename_link(&self, source: &Path, destination: &Path) -> Result<(), ManagedErrorCode> {
        fs::rename(source, destination).map_err(|_| ManagedErrorCode::BindingCreateFailed)
    }

    fn remove_link(&self, link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
        let inspection = self.inspect(link)?;
        match inspection {
            LinkInspection::Missing => Ok(()),
            LinkInspection::Link {
                kind: actual_kind, ..
            } if actual_kind == kind => remove_platform_link(link, kind),
            LinkInspection::Link { .. } | LinkInspection::Other => {
                Err(ManagedErrorCode::BindingConflict)
            }
        }
    }
}

pub const fn expected_link_kind(platform: Platform) -> Result<LinkKind, ManagedErrorCode> {
    match platform {
        Platform::Macos | Platform::Linux => Ok(LinkKind::Symlink),
        Platform::Windows => Ok(LinkKind::Junction),
        Platform::Unsupported => Err(ManagedErrorCode::BindingUnsupported),
    }
}

fn inspect_link(link: &Path) -> Result<LinkInspection, ManagedErrorCode> {
    #[cfg(target_os = "windows")]
    if junction::exists(link).map_err(|_| ManagedErrorCode::BindingVerifyFailed)? {
        let target =
            junction::get_target(link).map_err(|_| ManagedErrorCode::BindingVerifyFailed)?;
        return Ok(LinkInspection::Link {
            target_exists: target.exists(),
            kind: LinkKind::Junction,
            target,
        });
    }

    let metadata = match fs::symlink_metadata(link) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LinkInspection::Missing);
        }
        Err(_) => return Err(ManagedErrorCode::BindingVerifyFailed),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(LinkInspection::Other);
    }

    let raw_target = fs::read_link(link).map_err(|_| ManagedErrorCode::BindingVerifyFailed)?;
    let target = if raw_target.is_absolute() {
        raw_target
    } else {
        link.parent()
            .ok_or(ManagedErrorCode::BindingVerifyFailed)?
            .join(raw_target)
    };
    Ok(LinkInspection::Link {
        target_exists: target.exists(),
        kind: LinkKind::Symlink,
        target,
    })
}

#[cfg(unix)]
fn remove_platform_link(link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
    if kind != LinkKind::Symlink {
        return Err(ManagedErrorCode::BindingUnsupported);
    }
    fs::remove_file(link).map_err(|_| ManagedErrorCode::BindingVerifyFailed)
}

#[cfg(target_os = "windows")]
fn remove_platform_link(link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
    if kind != LinkKind::Junction {
        return Err(ManagedErrorCode::BindingUnsupported);
    }
    junction::delete(link).map_err(|_| ManagedErrorCode::BindingVerifyFailed)
}

#[cfg(not(any(unix, target_os = "windows")))]
fn remove_platform_link(_link: &Path, _kind: LinkKind) -> Result<(), ManagedErrorCode> {
    Err(ManagedErrorCode::BindingUnsupported)
}

fn has_unsafe_components(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
}

#[cfg(target_os = "windows")]
fn validate_windows_junction_paths(target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
    use std::path::Prefix;

    fn drive(path: &Path) -> Option<char> {
        match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                    Some((letter as char).to_ascii_uppercase())
                }
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) => None,
                _ => None,
            },
            _ => None,
        }
    }

    let target_drive = drive(target).ok_or(ManagedErrorCode::BindingUnsupported)?;
    let link_drive = drive(link).ok_or(ManagedErrorCode::BindingUnsupported)?;
    if target_drive != link_drive {
        return Err(ManagedErrorCode::BindingUnsupported);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_platform_has_one_explicit_link_kind() {
        assert_eq!(
            expected_link_kind(Platform::Macos).unwrap(),
            LinkKind::Symlink
        );
        assert_eq!(
            expected_link_kind(Platform::Linux).unwrap(),
            LinkKind::Symlink
        );
        assert_eq!(
            expected_link_kind(Platform::Windows).unwrap(),
            LinkKind::Junction
        );
        assert_eq!(
            expected_link_kind(Platform::Unsupported).unwrap_err(),
            ManagedErrorCode::BindingUnsupported
        );
    }

    #[test]
    fn relative_paths_are_rejected_before_link_creation() {
        let linker = SystemPlatformLinker::current();

        assert_eq!(
            linker
                .validate_paths(Path::new("relative-target"), Path::new("relative-link"))
                .unwrap_err(),
            ManagedErrorCode::ManagedPathOutsideRoot
        );
    }
}
