//! Functions for applying patches to a work directory.
use std::path::{Path, PathBuf};

use patch::Patch;

use super::SourceError;
use crate::system_tools::{SystemTools, Tool};

fn guess_strip_level(patch: &Path, work_dir: &Path) -> Result<usize, std::io::Error> {
    let text = std::fs::read_to_string(patch)?;
    let Ok(patches) = Patch::from_multiple(&text) else {
        return Ok(1);
    };

    // Try to guess the strip level by checking if the path exists in the work directory
    for p in patches {
        let path = PathBuf::from(p.old.path.as_ref());
        for strip_level in 0..path.components().count() {
            let mut new_path = work_dir.to_path_buf();
            new_path.extend(path.components().skip(strip_level));
            if new_path.exists() {
                return Ok(strip_level);
            }
        }
    }

    // If we can't guess the strip level, default to 1 (usually the patch file starts with a/ and b/)
    Ok(1)
}

/// Applies all patches in a list of patches to the specified work directory
/// Currently only supports patching with the `patch` command.
pub(crate) fn apply_patches(
    system_tools: &SystemTools,
    patches: &[PathBuf],
    work_dir: &Path,
    recipe_dir: &Path,
) -> Result<(), SourceError> {
    for patch in patches {
        let patch = recipe_dir.join(patch);

        let strip_level = guess_strip_level(&patch, work_dir)?;

        let output = system_tools
            .call(Tool::Patch)
            .map_err(|_| SourceError::PatchNotFound)?
            .arg(format!("-p{}", strip_level))
            .arg("-i")
            .arg(String::from(patch.to_string_lossy()))
            .arg("-d")
            .arg(String::from(work_dir.to_string_lossy()))
            .output()?;

        if !output.status.success() {
            tracing::error!("Failed to apply patch: {}", patch.to_string_lossy());
            tracing::error!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
            tracing::error!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
            return Err(SourceError::PatchFailed(
                patch.to_string_lossy().to_string(),
            ));
        }
    }
    Ok(())
}
