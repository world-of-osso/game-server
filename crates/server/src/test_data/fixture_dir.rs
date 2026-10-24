//! Disk-backed fixture directories retained by process-lifetime input caches.
use std::io;
use std::path::Path;

pub(crate) fn create_fixture_dir(root: &Path) -> io::Result<tempfile::TempDir> {
    std::fs::create_dir_all(root)?;
    tempfile::tempdir_in(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preparing_fixture_directory_removes_dead_pid_siblings_and_preserves_live_pid() {
        let root = tempfile::tempdir().unwrap();
        let dead_pid = u32::MAX;
        assert!(!Path::new("/proc").join(dead_pid.to_string()).exists());
        let abandoned = root.path().join(format!("{dead_pid}-abandoned"));
        std::fs::create_dir(&abandoned).unwrap();
        std::fs::write(abandoned.join("world.db"), b"abandoned fixture").unwrap();
        let live = root.path().join(format!("{}-live", std::process::id()));
        std::fs::create_dir(&live).unwrap();
        std::fs::write(live.join("world.db"), b"live fixture").unwrap();

        let prepared = create_fixture_dir(root.path()).unwrap();

        assert_eq!(
            std::fs::read(live.join("world.db")).unwrap(),
            b"live fixture"
        );
        assert!(
            !abandoned.exists(),
            "preparing a fixture must remove a dead-pid sibling and its database"
        );
        assert!(prepared.path().is_dir());
        assert!(
            prepared
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(&format!("{}-", std::process::id()))
        );
    }
}
