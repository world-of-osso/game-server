//! Private debug-peer harness only; never enabled in default or release builds.
#[cfg(not(debug_assertions))]
compile_error!("native-proof-hold-persistence is restricted to private debug proof builds");

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

const CONTROL_DIRECTORY: &str = "NATIVE_PROOF_HOLD_PERSISTENCE_DIR";
const MAXIMUM_HOLD: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

pub(crate) fn wait_for_release() -> Result<(), String> {
    let Some(root) = std::env::var_os(CONTROL_DIRECTORY) else {
        return Ok(());
    };
    hold_purchase(Path::new(&root), MAXIMUM_HOLD)
}

fn validate_directory(root: &Path) -> Result<(), String> {
    if !root.is_absolute() {
        return Err("native proof persistence control directory must be absolute".into());
    }
    let metadata = fs::metadata(root)
        .map_err(|error| format!("native proof control directory {}: {error}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "native proof control path {} is not a directory",
            root.display()
        ));
    }
    Ok(())
}

fn claim_arm(root: &Path) -> Result<bool, String> {
    let arm = root.join("arm");
    if !arm
        .try_exists()
        .map_err(|error| format!("native proof arm {}: {error}", arm.display()))?
    {
        return Ok(false);
    }
    for name in ["held", "release"] {
        let marker = root.join(name);
        if marker
            .try_exists()
            .map_err(|error| format!("native proof marker {}: {error}", marker.display()))?
        {
            return Err(format!(
                "native proof persistence stale {name} marker: {}",
                marker.display()
            ));
        }
    }
    fs::rename(&arm, root.join("held"))
        .map_err(|error| format!("native proof persistence claim {}: {error}", arm.display()))?;
    Ok(true)
}

fn wait_for_signal(root: &Path, maximum: Duration) -> Result<(), String> {
    let release = root.join("release");
    let deadline = Instant::now() + maximum;
    loop {
        if Instant::now() >= deadline {
            return Err(format!(
                "native proof persistence hold deadline expired after {}s without release: {}",
                maximum.as_secs_f64(),
                root.display()
            ));
        }
        if release
            .try_exists()
            .map_err(|error| format!("native proof release {}: {error}", release.display()))?
        {
            fs::remove_file(&release).map_err(|error| {
                format!(
                    "consume native proof release {}: {error}",
                    release.display()
                )
            })?;
            return Ok(());
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn remove_held_marker(root: &Path, result: Result<(), String>) -> Result<(), String> {
    let held = root.join("held");
    let cleanup = fs::remove_file(&held).map_err(|error| {
        format!(
            "remove native proof held marker {}: {error}",
            held.display()
        )
    });
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn hold_purchase(root: &Path, maximum: Duration) -> Result<(), String> {
    validate_directory(root)?;
    if !claim_arm(root)? {
        return Ok(());
    }
    eprintln!("NATIVE_PROOF_HOLD_PERSISTENCE held {}", root.display());
    let result = remove_held_marker(root, wait_for_signal(root, maximum));
    match &result {
        Ok(()) => eprintln!("NATIVE_PROOF_HOLD_PERSISTENCE released {}", root.display()),
        Err(error) => eprintln!("NATIVE_PROOF_HOLD_PERSISTENCE failed: {error}"),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_proof_hold_persistence_unarmed_directory_does_not_wait() {
        let dir = tempfile::tempdir().unwrap();
        hold_purchase(dir.path(), Duration::ZERO).unwrap();
        assert!(!dir.path().join("held").exists());
        assert!(!dir.path().join("release").exists());
    }

    #[test]
    fn native_proof_hold_persistence_stale_signals_fail_without_consuming_arm() {
        for marker in ["held", "release"] {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("arm"), b"one request").unwrap();
            fs::write(dir.path().join(marker), b"stale").unwrap();
            let error = hold_purchase(dir.path(), Duration::ZERO).unwrap_err();
            assert!(error.contains(&format!("stale {marker}")));
            assert!(dir.path().join("arm").exists());
        }
    }

    #[test]
    fn native_proof_hold_persistence_deadline_cleans_hold_without_release() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("arm"), b"one request").unwrap();
        let error = hold_purchase(dir.path(), Duration::ZERO).unwrap_err();
        assert!(error.contains("deadline expired"));
        assert!(!dir.path().join("arm").exists());
        assert!(!dir.path().join("held").exists());
        assert!(!dir.path().join("release").exists());
    }

    #[test]
    fn native_proof_hold_persistence_invalid_directory_is_explicit() {
        assert!(
            hold_purchase(Path::new("relative"), Duration::ZERO)
                .unwrap_err()
                .contains("absolute")
        );
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");
        assert!(
            hold_purchase(&missing, Duration::ZERO)
                .unwrap_err()
                .contains("control directory")
        );
    }
}
