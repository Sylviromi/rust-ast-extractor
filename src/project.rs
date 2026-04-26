use std::path::{Path, PathBuf};

pub fn find_project_root(start: &Path) -> PathBuf {
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if current.join("Cargo.toml").exists() || current.join(".git").exists() {
            return current;
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return std::env::current_dir().unwrap_or_else(|_| start.to_path_buf()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn finds_root_via_cargo_toml() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let src = tmp.path().join("src");
        fs::create_dir(&src).unwrap();

        let root = find_project_root(&src);
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn finds_root_via_git() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        let deep = tmp.path().join("a").join("b");
        fs::create_dir_all(&deep).unwrap();

        let root = find_project_root(&deep);
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn falls_back_to_a_directory_when_no_marker() {
        let tmp = TempDir::new().unwrap();
        let root = find_project_root(tmp.path());
        // Falls back to cwd — must be a directory, not a file
        assert!(root.is_dir());
    }
}
