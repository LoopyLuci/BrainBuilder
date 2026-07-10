// Real checkpoint version history: training the same graph twice used to be
// a one-way door — the new checkpoint silently overwrote the old one with no
// way back. Now, right before a fresh checkpoint would overwrite the current
// one, the current file is archived under a timestamp id first, so a bad
// retrain is always recoverable.
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckpointVersion {
    /// Milliseconds since the Unix epoch when this version was archived —
    /// also the filename stem, and sortable as a plain string.
    pub id: String,
    pub size_bytes: u64,
}

fn checkpoint_path(checkpoints_dir: &Path, graph_id: &str) -> PathBuf {
    checkpoints_dir.join(format!("{graph_id}.pt"))
}

fn versions_dir(checkpoints_dir: &Path, graph_id: &str) -> PathBuf {
    checkpoints_dir.join("versions").join(graph_id)
}

/// Picks a millis-since-epoch id for a new archive entry, bumping by one
/// millisecond at a time when that id is already taken. Two archives can
/// legitimately land in the same millisecond — `restore_version` archives
/// the checkpoint it's about to replace mere microseconds after a caller
/// may have just archived one itself — and colliding on a filename would
/// silently overwrite (and corrupt) an existing archived version instead of
/// creating a new one, rather than erroring loudly.
fn unique_archive_path(dir: &Path) -> Result<PathBuf> {
    let mut millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| BrainBuilderError::ConfigError(format!("system clock error: {e}")))?
        .as_millis();
    loop {
        let candidate = dir.join(format!("{millis}.pt"));
        if !candidate.exists() {
            return Ok(candidate);
        }
        millis += 1;
    }
}

/// Archives the current checkpoint for `graph_id`, if one exists, before it
/// would otherwise be overwritten. A no-op (not an error) when there's
/// nothing to archive yet — a graph's first-ever training run.
pub fn archive_current(checkpoints_dir: &Path, graph_id: &str) -> Result<()> {
    let current = checkpoint_path(checkpoints_dir, graph_id);
    if !current.exists() {
        return Ok(());
    }
    let dir = versions_dir(checkpoints_dir, graph_id);
    std::fs::create_dir_all(&dir)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to create version history folder: {e}")))?;
    let dest = unique_archive_path(&dir)?;
    std::fs::copy(&current, &dest)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to archive the current checkpoint: {e}")))?;
    Ok(())
}

/// Every archived version for a graph, newest first. Empty (not an error)
/// for a graph that's never been retrained over an existing checkpoint.
pub fn list_versions(checkpoints_dir: &Path, graph_id: &str) -> Result<Vec<CheckpointVersion>> {
    let dir = versions_dir(checkpoints_dir, graph_id);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut versions: Vec<CheckpointVersion> = std::fs::read_dir(&dir)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to read version history: {e}")))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pt"))
        .filter_map(|entry| {
            let id = entry.path().file_stem()?.to_str()?.to_string();
            let size_bytes = entry.metadata().ok()?.len();
            Some(CheckpointVersion { id, size_bytes })
        })
        .collect();
    // Millis-since-epoch ids are all the same digit count for a very long
    // time (through the year 2286), so plain string sort matches numeric
    // sort — newest (largest) first.
    versions.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(versions)
}

/// Restores an archived version as the current checkpoint. The checkpoint
/// being replaced is archived first (via `archive_current`), so restoring is
/// itself just as reversible as the training run that prompted it — never a
/// one-way door either.
pub fn restore_version(checkpoints_dir: &Path, graph_id: &str, version_id: &str) -> Result<()> {
    let source = versions_dir(checkpoints_dir, graph_id).join(format!("{version_id}.pt"));
    if !source.exists() {
        return Err(BrainBuilderError::ConfigError(format!(
            "no archived version `{version_id}` exists for this graph"
        )));
    }
    archive_current(checkpoints_dir, graph_id)?;
    let dest = checkpoint_path(checkpoints_dir, graph_id);
    std::fs::copy(&source, &dest)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to restore the archived version: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_checkpoints_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bb_checkpoint_versions_{name}_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_checkpoint(dir: &Path, graph_id: &str, content: &[u8]) {
        std::fs::write(checkpoint_path(dir, graph_id), content).unwrap();
    }

    #[test]
    fn archiving_with_no_existing_checkpoint_is_a_harmless_noop() {
        let dir = temp_checkpoints_dir("noop");
        archive_current(&dir, "g1").unwrap();
        assert!(list_versions(&dir, "g1").unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn archiving_snapshots_the_existing_checkpoint() {
        let dir = temp_checkpoints_dir("snapshot");
        write_checkpoint(&dir, "g1", b"version one bytes");
        archive_current(&dir, "g1").unwrap();

        let versions = list_versions(&dir, "g1").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].size_bytes, "version one bytes".len() as u64);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restoring_brings_back_older_content_and_archives_what_it_replaced() {
        let dir = temp_checkpoints_dir("restore");
        write_checkpoint(&dir, "g1", b"v1");
        archive_current(&dir, "g1").unwrap(); // archives v1
        write_checkpoint(&dir, "g1", b"v2-newer-and-different");

        let versions_before = list_versions(&dir, "g1").unwrap();
        assert_eq!(versions_before.len(), 1, "only v1 archived so far");
        let v1_id = versions_before[0].id.clone();

        restore_version(&dir, "g1", &v1_id).unwrap();

        let restored = std::fs::read(checkpoint_path(&dir, "g1")).unwrap();
        assert_eq!(restored, b"v1", "the current checkpoint should be v1's content again");

        let versions_after = list_versions(&dir, "g1").unwrap();
        assert_eq!(versions_after.len(), 2, "restoring should have archived v2 before overwriting it");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unique_archive_path_never_collides_even_when_called_back_to_back() {
        // Regression test: two archives landing in the same millisecond used
        // to silently overwrite each other's file (see restore_version's
        // archive-then-copy sequence, which does exactly this in practice).
        let dir = temp_checkpoints_dir("collision");
        let first = unique_archive_path(&dir).unwrap();
        std::fs::write(&first, b"first").unwrap();
        let second = unique_archive_path(&dir).unwrap();
        assert_ne!(first, second, "a second call before the first path exists on disk must not reuse it");
        std::fs::write(&second, b"second").unwrap();

        assert_eq!(std::fs::read(&first).unwrap(), b"first", "the first archive must survive untouched");
        assert_eq!(std::fs::read(&second).unwrap(), b"second");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restoring_an_unknown_version_errors_clearly() {
        let dir = temp_checkpoints_dir("missing");
        write_checkpoint(&dir, "g1", b"v1");
        let err = restore_version(&dir, "g1", "not-a-real-id").unwrap_err();
        assert!(err.to_string().contains("no archived version"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
