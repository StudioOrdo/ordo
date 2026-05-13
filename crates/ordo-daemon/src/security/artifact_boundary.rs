use anyhow::{bail, Context, Result};
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

pub(crate) fn resolve_existing_artifact_path(
    boundary: impl AsRef<Path>,
    candidate: impl AsRef<Path>,
    label: &str,
) -> Result<PathBuf> {
    let boundary = canonical_artifact_boundary(boundary.as_ref(), label)?;
    let candidate = candidate.as_ref();
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        boundary.join(candidate)
    };
    let path = path
        .canonicalize()
        .with_context(|| format!("{label} is not accessible"))?;
    ensure_artifact_path_within(&path, &boundary, label)?;
    Ok(path)
}

pub(crate) fn resolve_artifact_output_path(
    boundary: impl AsRef<Path>,
    candidate: impl AsRef<Path>,
    label: &str,
) -> Result<PathBuf> {
    let boundary = canonical_artifact_boundary(boundary.as_ref(), label)?;
    let candidate = candidate.as_ref();
    ensure_relative_artifact_path(candidate, label)?;
    let parent = candidate.parent().unwrap_or_else(|| Path::new(""));
    let parent = if parent.as_os_str().is_empty() {
        boundary.clone()
    } else {
        resolve_existing_artifact_path(&boundary, parent, label)?
    };
    let file_name = candidate
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("{label} is missing a file name"))?;
    let path = parent.join(file_name);
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("{label} cannot target a symlink")
        }
        Ok(_) => {
            let canonical_path = path
                .canonicalize()
                .with_context(|| format!("{label} is not accessible"))?;
            ensure_artifact_path_within(&canonical_path, &boundary, label)?;
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("{label} is not accessible")),
    }
    ensure_artifact_path_within(&path, &boundary, label)?;
    Ok(path)
}

pub(crate) fn ensure_artifact_path_within(path: &Path, boundary: &Path, label: &str) -> Result<()> {
    if !path.starts_with(boundary) {
        bail!("{label} escapes artifact boundary");
    }
    Ok(())
}

fn canonical_artifact_boundary(boundary: &Path, label: &str) -> Result<PathBuf> {
    let boundary = boundary
        .canonicalize()
        .with_context(|| format!("{label} boundary is not accessible"))?;
    if !boundary.is_dir() {
        bail!("{label} boundary is not a directory");
    }
    Ok(boundary)
}

fn ensure_relative_artifact_path(path: &Path, label: &str) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("{label} is empty");
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => bail!("{label} contains parent traversal"),
            Component::RootDir | Component::Prefix(_) => bail!("{label} must be relative"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolves_existing_relative_path_inside_boundary() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nested = temp_dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        let artifact = nested.join("artifact.json");
        fs::write(&artifact, "{}").unwrap();

        let resolved =
            resolve_existing_artifact_path(temp_dir.path(), "nested/artifact.json", "packetPath")
                .unwrap();

        assert_eq!(resolved, artifact.canonicalize().unwrap());
    }

    #[test]
    fn rejects_parent_traversal_escape() {
        let boundary = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();

        let error = resolve_existing_artifact_path(
            boundary.path(),
            Path::new("..")
                .join(outside.path().file_name().unwrap())
                .join("secret.txt"),
            "packetPath",
        )
        .unwrap_err();

        let message = error.to_string();
        assert!(
            message.contains("escapes artifact boundary")
                || message.contains("packetPath is not accessible")
        );
    }

    #[test]
    fn rejects_absolute_path_escape() {
        let boundary = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();

        let error = resolve_existing_artifact_path(boundary.path(), &outside_file, "packetPath")
            .unwrap_err();

        assert!(error.to_string().contains("escapes artifact boundary"));
    }

    #[test]
    fn resolves_unicode_output_path_inside_boundary() {
        let boundary = tempfile::tempdir().unwrap();
        let nested = boundary.path().join("out");
        fs::create_dir(&nested).unwrap();

        let resolved =
            resolve_artifact_output_path(boundary.path(), "out/résumé.md", "artifact output")
                .unwrap();

        assert_eq!(resolved, nested.canonicalize().unwrap().join("résumé.md"));
    }

    #[test]
    fn rejects_output_parent_traversal() {
        let boundary = tempfile::tempdir().unwrap();

        let error =
            resolve_artifact_output_path(boundary.path(), "../secret.md", "artifact output")
                .unwrap_err();

        assert!(error.to_string().contains("contains parent traversal"));
    }

    #[test]
    fn resolves_existing_regular_output_path_inside_boundary() {
        let boundary = tempfile::tempdir().unwrap();
        let output = boundary.path().join("artifact.md");
        fs::write(&output, "previous").unwrap();

        let resolved =
            resolve_artifact_output_path(boundary.path(), "artifact.md", "artifact output")
                .unwrap();

        assert_eq!(
            resolved.canonicalize().unwrap(),
            output.canonicalize().unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let boundary = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();
        symlink(outside.path(), boundary.path().join("link")).unwrap();

        let error =
            resolve_existing_artifact_path(boundary.path(), "link/secret.txt", "packetPath")
                .unwrap_err();

        assert!(error.to_string().contains("escapes artifact boundary"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_output_symlink_escape() {
        use std::os::unix::fs::symlink;

        let boundary = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("secret.md");
        fs::write(&outside_file, "secret").unwrap();
        symlink(&outside_file, boundary.path().join("artifact.md")).unwrap();

        let error = resolve_artifact_output_path(boundary.path(), "artifact.md", "artifact output")
            .unwrap_err();

        assert!(error.to_string().contains("cannot target a symlink"));
    }
}
