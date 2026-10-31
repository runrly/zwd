use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    error::{AppError, Result},
    workspace::{LinkName, ResolvedFolder},
};

const LOCK_FILE: &str = ".zwd-lock.json";
const LOCK_VERSION: u8 = 1;

#[derive(Debug, Deserialize, Serialize)]
struct DockLock {
    version: u8,
    workspace_path: PathBuf,
    links: Vec<DockLink>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DockLink {
    name: String,
    target: PathBuf,
}

#[derive(Debug)]
pub(crate) enum DockStatus {
    Absent { path: PathBuf },
    Healthy { path: PathBuf },
    Unhealthy { path: PathBuf, reason: String },
}

pub(crate) fn build_dock(workspace_path: &Path, folders: &[ResolvedFolder]) -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().ok_or(AppError::CacheDirNotFound)?;
    build_dock_in(&cache_dir, workspace_path, folders)
}

pub(crate) fn build_dock_in(
    cache_dir: &Path,
    workspace_path: &Path,
    folders: &[ResolvedFolder],
) -> Result<PathBuf> {
    let workspace_abs = absolute_workspace_path(workspace_path)?;
    let dock_root = dock_root_in(cache_dir, workspace_path, &workspace_abs);

    if metadata(&dock_root)?.is_none() {
        fs::create_dir_all(&dock_root)?;
        write_lock(&dock_root, &workspace_abs, &[])?;
    }
    reconcile_dock(&dock_root, &workspace_abs, folders)?;

    Ok(dock_root)
}

pub(crate) fn infer_workspace_from_current_dock() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    if !current_dir.join(LOCK_FILE).exists() {
        return Err(AppError::WorkspaceReferenceRequired);
    }

    let lock = read_lock(&current_dir)?;
    let workspace_path = absolute_workspace_path(&lock.workspace_path)?;
    let expected_dock = dock_root_for(&workspace_path)?;
    if current_dir != expected_dock {
        return Err(AppError::WorkspaceReferenceRequired);
    }

    validate_owned_dock(&current_dir, &workspace_path)?;

    Ok(workspace_path)
}

pub(crate) fn validate_existing_dock(workspace_path: &Path) -> Result<Option<PathBuf>> {
    let dock_root = dock_root_for(workspace_path)?;
    if metadata(&dock_root)?.is_none() {
        return Ok(None);
    }

    let workspace_abs = absolute_workspace_path(workspace_path)?;
    validate_owned_dock(&dock_root, &workspace_abs)?;

    Ok(Some(dock_root))
}

pub(crate) fn sync_existing_dock(
    workspace_path: &Path,
    folders: &[ResolvedFolder],
) -> Result<bool> {
    let Some(dock_root) = validate_existing_dock(workspace_path)? else {
        return Ok(false);
    };
    let workspace_abs = absolute_workspace_path(workspace_path)?;
    reconcile_dock(&dock_root, &workspace_abs, folders)?;

    Ok(true)
}

pub(crate) fn remove_owned_dock(workspace_path: &Path) -> Result<bool> {
    let Some(dock_root) = validate_existing_dock(workspace_path)? else {
        return Ok(false);
    };
    fs::remove_dir_all(&dock_root)?;

    Ok(true)
}

pub(crate) fn current_directory_is_dock(workspace_path: &Path) -> Result<bool> {
    let dock = dock_root_for(workspace_path)?;
    let Some(_) = metadata(&dock)? else {
        return Ok(false);
    };
    Ok(std::env::current_dir()?.starts_with(dock.canonicalize()?))
}

pub(crate) fn dock_status(workspace_path: &Path) -> Result<DockStatus> {
    let dock_root = dock_root_for(workspace_path)?;
    if metadata(&dock_root)?.is_none() {
        return Ok(DockStatus::Absent { path: dock_root });
    }

    let workspace_abs = absolute_workspace_path(workspace_path)?;
    match inspect_dock_health(&dock_root, &workspace_abs) {
        Ok(_) => Ok(DockStatus::Healthy { path: dock_root }),
        Err(error) => Ok(DockStatus::Unhealthy {
            path: dock_root,
            reason: error.to_string(),
        }),
    }
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn validate_owned_dock(dock_root: &Path, workspace_path: &Path) -> Result<DockLock> {
    if !fs::symlink_metadata(dock_root)?.is_dir() {
        return Err(AppError::DockPathNotDirectory {
            path: dock_root.to_path_buf(),
        });
    }
    let lock = read_lock(dock_root)?;
    if lock.workspace_path != workspace_path {
        return Err(AppError::DockWorkspacePathMismatch {
            lock_workspace_path: lock.workspace_path,
            workspace_path: workspace_path.to_path_buf(),
        });
    }
    Ok(lock)
}

fn validate_managed_links(dock_root: &Path, lock: &DockLock) -> Result<()> {
    for link in &lock.links {
        let path = dock_root.join(&link.name);
        let Some(entry) = metadata(&path)? else {
            continue;
        };
        if !entry.is_symlink() {
            return Err(AppError::ManagedDockEntryNotSymlink { path });
        }
        let actual = fs::read_link(&path)?;
        if actual != link.target {
            return Err(AppError::DockLinkTargetMismatch {
                path,
                expected: link.target.clone(),
                actual,
            });
        }
    }
    Ok(())
}

fn inspect_dock_health(dock_root: &Path, workspace_path: &Path) -> Result<()> {
    let lock = validate_owned_dock(dock_root, workspace_path)?;
    validate_managed_links(dock_root, &lock)?;
    let folders = crate::workspace::read_workspace_file(workspace_path)?
        .resolved_dock_folders(workspace_path)?;
    if lock.links.len() != folders.len()
        || folders.iter().any(|folder| {
            !lock
                .links
                .iter()
                .any(|link| link.name == folder.name.as_str() && link.target == folder.target)
        })
    {
        return Err(AppError::DockOutOfSync {
            path: dock_root.to_path_buf(),
        });
    }
    for link in &lock.links {
        if metadata(&dock_root.join(&link.name))?.is_none() {
            return Err(AppError::DockOutOfSync {
                path: dock_root.join(&link.name),
            });
        }
    }
    Ok(())
}

struct LinkChange {
    path: PathBuf,
    previous: Option<PathBuf>,
    desired: Option<PathBuf>,
}

fn reconcile_dock(
    dock_root: &Path,
    workspace_path: &Path,
    folders: &[ResolvedFolder],
) -> Result<()> {
    let lock = validate_owned_dock(dock_root, workspace_path)?;
    validate_managed_links(dock_root, &lock)?;
    let mut changes = Vec::new();
    for link in &lock.links {
        let path = dock_root.join(&link.name);
        let present = metadata(&path)?.is_some();
        let desired = folders
            .iter()
            .find(|folder| folder.name.as_str() == link.name);
        if present && desired.is_none_or(|folder| folder.target != link.target) {
            changes.push(LinkChange {
                path,
                previous: Some(link.target.clone()),
                desired: desired.map(|folder| folder.target.clone()),
            });
        }
    }
    for folder in folders {
        let path = dock_root.join(folder.name.as_str());
        let present = metadata(&path)?.is_some();
        let managed = lock
            .links
            .iter()
            .any(|link| link.name == folder.name.as_str());
        if present && !managed {
            return Err(AppError::DockLinkPathExists { path });
        }
        if !present {
            changes.push(LinkChange {
                path,
                previous: None,
                desired: Some(folder.target.clone()),
            });
        }
    }
    for (index, change) in changes.iter().enumerate() {
        if let Err(error) = apply_link_change(change) {
            restore_links(&changes[..=index]);
            return Err(error);
        }
    }
    if let Err(error) = write_lock(dock_root, workspace_path, folders) {
        restore_links(&changes);
        return Err(error);
    }
    Ok(())
}

fn apply_link_change(change: &LinkChange) -> Result<()> {
    if change.previous.is_some() {
        remove_managed_link(&change.path)?;
    }
    if let Some(target) = &change.desired {
        create_symlink(target, &change.path)?;
    }
    Ok(())
}

fn restore_links(changes: &[LinkChange]) {
    for change in changes.iter().rev() {
        // Only undo links created by this reconciliation; never remove a conflicting entry.
        if change
            .desired
            .as_ref()
            .is_some_and(|target| fs::read_link(&change.path).ok().as_ref() == Some(target))
        {
            let _ = remove_managed_link(&change.path);
        }
        if let Some(target) = &change.previous
            && matches!(metadata(&change.path), Ok(None))
        {
            let _ = create_symlink(target, &change.path);
        }
    }
}

fn read_lock(dock_root: &Path) -> Result<DockLock> {
    let lock_path = dock_root.join(LOCK_FILE);

    let Some(lock_metadata) = metadata(&lock_path)? else {
        return Err(AppError::DockMissingLock {
            path: dock_root.to_path_buf(),
        });
    };
    if !lock_metadata.is_file() {
        return Err(AppError::InvalidDockLock {
            path: lock_path,
            reason: "must be a regular file, not a symlink",
        });
    }
    let content = fs::read_to_string(&lock_path)?;
    let lock: DockLock = serde_json::from_str(&content)?;

    if lock.version != LOCK_VERSION {
        return Err(AppError::UnsupportedDockLockVersion {
            version: lock.version,
        });
    }

    if !lock.workspace_path.is_absolute()
        || lock.links.iter().any(|link| !link.target.is_absolute())
    {
        return Err(AppError::InvalidDockLock {
            path: lock_path,
            reason: "workspace and target paths must be absolute",
        });
    }
    let mut names = HashSet::new();
    for link in &lock.links {
        LinkName::new(&link.name)?;
        if !names.insert(link.name.to_lowercase()) {
            return Err(AppError::DuplicateFolderName {
                name: link.name.clone(),
            });
        }
    }
    Ok(lock)
}

fn write_lock(dock_root: &Path, workspace_path: &Path, folders: &[ResolvedFolder]) -> Result<()> {
    let lock = DockLock {
        version: LOCK_VERSION,
        workspace_path: workspace_path.to_path_buf(),
        links: folders
            .iter()
            .map(|folder| DockLink {
                name: folder.name.as_str().to_string(),
                target: folder.target.clone(),
            })
            .collect(),
    };
    let content = serde_json::to_string_pretty(&lock)?;

    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce)?;
    let temporary_path = dock_root.join(format!(".zwd-lock-{}.tmp", hex::encode(nonce)));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)?;
    let result = (|| -> Result<()> {
        writeln!(file, "{content}")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, dock_root.join(LOCK_FILE))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary_path);
    }
    result
}

fn absolute_workspace_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(fs::canonicalize(path)?);
    }

    let current_dir = std::env::current_dir()?;
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    })
}

fn dock_root_for(workspace_path: &Path) -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().ok_or(AppError::CacheDirNotFound)?;
    let workspace_abs = absolute_workspace_path(workspace_path)?;

    Ok(dock_root_in(&cache_dir, workspace_path, &workspace_abs))
}

fn dock_root_in(cache_dir: &Path, workspace_path: &Path, workspace_abs: &Path) -> PathBuf {
    cache_dir
        .join("zwd")
        .join("docks")
        .join(dock_name(workspace_path, workspace_abs))
}

fn dock_name(workspace_path: &Path, workspace_abs: &Path) -> String {
    let stem = workspace_path
        .file_stem()
        .map(|stem| stem.to_string_lossy())
        .unwrap_or_else(|| "workspace".into());
    let slug = slugify(&stem);
    let mut hasher = Sha256::new();
    hasher.update(workspace_abs.to_string_lossy().as_bytes());
    let hash = hex::encode(hasher.finalize());

    format!("{slug}-{}", &hash[..12])
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "workspace".to_string()
    } else {
        slug
    }
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    if link.exists() || link.symlink_metadata().is_ok() {
        return Err(AppError::DockLinkPathExists {
            path: link.to_path_buf(),
        });
    }

    std::os::unix::fs::symlink(target, link)?;

    Ok(())
}

#[cfg(windows)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    if link.exists() || link.symlink_metadata().is_ok() {
        return Err(AppError::DockLinkPathExists {
            path: link.to_path_buf(),
        });
    }

    std::os::windows::fs::symlink_dir(target, link).map_err(|source| AppError::WindowsSymlink {
        path: link.to_path_buf(),
        source,
    })?;

    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<()> {
    Err(AppError::UnsupportedSymlinkPlatform)
}

#[cfg(windows)]
fn remove_managed_link(path: &Path) -> Result<()> {
    fs::remove_dir(path)?;

    Ok(())
}

#[cfg(not(windows))]
fn remove_managed_link(path: &Path) -> Result<()> {
    fs::remove_file(path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::Value;
    use tempfile::tempdir;

    use super::*;
    use crate::workspace::LinkName;

    const LOCK_SCHEMA: &str = include_str!("../resources/schemas/zwd-lock.schema.json");

    #[test]
    fn builds_dock_with_symlinks_and_lock() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("api");
        fs::create_dir(&project).unwrap();
        let workspace = temp.path().join("demo.code-workspace");
        fs::write(&workspace, "{}").unwrap();

        let dock_root = build_dock_in(
            temp.path(),
            &workspace,
            &[ResolvedFolder {
                name: LinkName::new("api").unwrap(),
                target: project.clone(),
            }],
        )
        .unwrap();

        assert!(dock_root.join(LOCK_FILE).exists());
        assert!(
            dock_root
                .join("api")
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(read_link_target(&dock_root.join("api")), project);
    }

    #[test]
    fn lock_schema_tracks_current_lock_version() {
        let schema: Value = serde_json::from_str(LOCK_SCHEMA).unwrap();

        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["properties"]["version"]["const"], LOCK_VERSION);
    }

    #[test]
    fn generated_lock_matches_published_schema_shape() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("api");
        fs::create_dir(&project).unwrap();
        let workspace = temp.path().join("demo.code-workspace");
        fs::write(&workspace, "{}").unwrap();

        let dock_root = build_dock_in(
            temp.path(),
            &workspace,
            &[ResolvedFolder {
                name: LinkName::new("api").unwrap(),
                target: project.clone(),
            }],
        )
        .unwrap();
        let lock: Value =
            serde_json::from_str(&fs::read_to_string(dock_root.join(LOCK_FILE)).unwrap()).unwrap();
        let lock = lock.as_object().expect("lock must be a JSON object");
        let links = lock["links"].as_array().expect("links must be an array");
        let first_link = links[0].as_object().expect("link must be a JSON object");

        assert_eq!(lock.len(), 3);
        assert_eq!(lock["version"], LOCK_VERSION);
        assert!(
            lock["workspace_path"]
                .as_str()
                .is_some_and(|path| !path.is_empty())
        );
        assert_eq!(first_link.len(), 2);
        assert_eq!(first_link["name"], "api");
        assert_eq!(first_link["target"], project.to_string_lossy().into_owned());
    }

    #[test]
    fn rebuilds_dock_idempotently() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("api");
        fs::create_dir(&project).unwrap();
        let workspace = temp.path().join("demo.code-workspace");
        fs::write(&workspace, "{}").unwrap();
        let folders = [ResolvedFolder {
            name: LinkName::new("api").unwrap(),
            target: project,
        }];

        let first = build_dock_in(temp.path(), &workspace, &folders).unwrap();
        let second = build_dock_in(temp.path(), &workspace, &folders).unwrap();

        assert_eq!(first, second);
        assert!(
            second
                .join("api")
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn aborts_when_lock_belongs_to_another_workspace() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("api");
        fs::create_dir(&project).unwrap();
        let workspace = temp.path().join("demo.code-workspace");
        let other_workspace = temp.path().join("other.code-workspace");
        fs::write(&workspace, "{}").unwrap();
        fs::write(&other_workspace, "{}").unwrap();
        let folders = [ResolvedFolder {
            name: LinkName::new("api").unwrap(),
            target: project.clone(),
        }];
        let dock_root = build_dock_in(temp.path(), &workspace, &folders).unwrap();
        let lock = serde_json::json!({
            "version": LOCK_VERSION,
            "workspace_path": other_workspace.canonicalize().unwrap(),
            "links": [{ "name": "api", "target": project }]
        });
        fs::write(
            dock_root.join(LOCK_FILE),
            format!("{}\n", serde_json::to_string_pretty(&lock).unwrap()),
        )
        .unwrap();

        let error = build_dock_in(temp.path(), &workspace, &folders)
            .unwrap_err()
            .to_string();

        assert!(error.contains("dock lock belongs"));
    }

    #[test]
    fn aborts_when_dock_exists_without_lock() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("api");
        fs::create_dir(&project).unwrap();
        let workspace = temp.path().join("demo.code-workspace");
        fs::write(&workspace, "{}").unwrap();
        let dock_root = temp
            .path()
            .join("zwd")
            .join("docks")
            .join(dock_name(&workspace, &workspace.canonicalize().unwrap()));
        fs::create_dir_all(&dock_root).unwrap();

        let error = build_dock_in(
            temp.path(),
            &workspace,
            &[ResolvedFolder {
                name: LinkName::new("api").unwrap(),
                target: project,
            }],
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("without lock"));
    }

    #[test]
    fn preserves_unmanaged_content_when_reconciling() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("api");
        fs::create_dir(&project).unwrap();
        let workspace = temp.path().join("demo.code-workspace");
        fs::write(&workspace, "{}").unwrap();
        let folders = [ResolvedFolder {
            name: LinkName::new("api").unwrap(),
            target: project,
        }];
        let dock_root = build_dock_in(temp.path(), &workspace, &folders).unwrap();
        fs::write(dock_root.join("notes.txt"), "do not delete").unwrap();

        build_dock_in(temp.path(), &workspace, &folders).unwrap();
        assert!(dock_root.join("notes.txt").exists());
    }

    #[test]
    fn failed_lock_replacement_preserves_destination_and_cleans_temporary_file() {
        let temp = tempdir().unwrap();
        let lock = temp.path().join(LOCK_FILE);
        fs::create_dir(&lock).unwrap();
        fs::write(lock.join("sentinel"), "keep").unwrap();
        assert!(write_lock(temp.path(), Path::new("/workspace.code-workspace"), &[]).is_err());
        assert_eq!(fs::read_to_string(lock.join("sentinel")).unwrap(), "keep");
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    fn read_link_target(path: &Path) -> PathBuf {
        fs::read_link(path).unwrap()
    }
}
