//! Startup-owned Toy casting data generated from the current canonical DB2 export.
use std::{collections::HashMap, path::Path};

use bevy::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};

#[derive(Deserialize)]
pub(super) struct AreaRequirements {
    pub build: String,
    pub spells: HashMap<u32, u32>,
    pub groups: HashMap<u32, Vec<u32>>,
}

#[derive(Deserialize)]
pub(super) struct FacingRequirements {
    pub build: String,
    pub spells: HashMap<u32, u32>,
    pub model_radii: HashMap<u32, f32>,
}

#[derive(Resource)]
pub(crate) struct ToyCastingRequirements {
    pub(super) areas: AreaRequirements,
    pub(super) facing: FacingRequirements,
}

fn read_requirements<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let file = std::fs::File::open(path).map_err(|error| {
        format!(
            "Read generated Toy requirements {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_reader(file).map_err(|error| {
        format!(
            "Parse generated Toy requirements {}: {error}",
            path.display()
        )
    })
}

pub(crate) fn load(data_dir: &Path) -> Result<ToyCastingRequirements, String> {
    let areas: AreaRequirements = read_requirements(&data_dir.join("toy_required_areas.json"))?;
    let facing: FacingRequirements =
        read_requirements(&data_dir.join("toy_facing_requirements.json"))?;
    if areas.build.is_empty() || facing.build.is_empty() {
        return Err("Generated Toy requirements have no source export build".into());
    }
    if !areas.spells.values().all(|group| {
        areas
            .groups
            .get(group)
            .is_some_and(|areas| !areas.is_empty())
    }) {
        return Err("Generated Toy area group has no members".into());
    }
    info!(
        "Loaded Toy casting requirements: areas build {}, facing build {}",
        areas.build, facing.build
    );
    Ok(ToyCastingRequirements { areas, facing })
}

#[cfg(any(test, feature = "test-support"))]
pub(super) fn fixture() -> ToyCastingRequirements {
    ToyCastingRequirements {
        areas: serde_json::from_str(include_str!("../../tests/fixtures/toy_required_areas.json"))
            .unwrap(),
        facing: serde_json::from_str(
            r#"{"build":"fixture","spells":{"225826":1,"406870":1},"model_radii":{"42247":6.0}}"#,
        )
        .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_inputs(path: &Path, build: &str, facing_flags: u32) {
        std::fs::write(
            path.join("toy_required_areas.json"),
            serde_json::json!({
                "build":build,"spells":{"61810":1103},"groups":{"1103":[3576]}
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            path.join("toy_facing_requirements.json"),
            serde_json::json!({
                "build":build,"spells":{"406870":facing_flags},"model_radii":{"42247":6.0}
            })
            .to_string(),
        )
        .unwrap();
    }

    #[test]
    fn toydata27_missing_files_fail_with_the_required_path() {
        let directory = tempfile::tempdir().unwrap();
        let error = load(directory.path()).err().unwrap();
        assert!(error.contains("toy_required_areas.json"), "{error}");
        write_inputs(directory.path(), "12.1.5.70391", 1);
        std::fs::remove_file(directory.path().join("toy_facing_requirements.json")).unwrap();
        let error = load(directory.path()).err().unwrap();
        assert!(error.contains("toy_facing_requirements.json"), "{error}");
    }

    #[test]
    fn toydata27_fresh_app_reads_changed_external_build_and_facing_behavior() {
        use shared::components::Rotation;
        let directory = tempfile::tempdir().unwrap();
        let cast = |data| {
            let mut app = App::new();
            app.insert_resource(data);
            let caster = super::super::test_harness::spawn_player(&mut app, 8, 0.0);
            app.world_mut().entity_mut(caster).insert(Rotation {
                x: 0.0,
                y: std::f32::consts::FRAC_PI_2,
                z: 0.0,
            });
            let target = super::super::test_harness::spawn_player(&mut app, 8, -3.0);
            super::super::toy_facing::check_cast(app.world(), caster, 406870, Some(target))
        };
        write_inputs(directory.path(), "12.1.5.70391", 0);
        let first = load(directory.path()).unwrap();
        assert_eq!(first.areas.build, "12.1.5.70391");
        assert!(cast(first).is_ok());
        write_inputs(directory.path(), "12.1.6.71000", 1);
        let second = load(directory.path()).unwrap();
        assert_eq!(second.facing.build, "12.1.6.71000");
        assert!(matches!(
            cast(second),
            Err(super::super::CastFailure::NotInFront)
        ));
    }

    #[test]
    fn toydata27_invalid_json_and_empty_area_groups_fail_loudly() {
        let directory = tempfile::tempdir().unwrap();
        write_inputs(directory.path(), "12.1.5.70391", 1);
        std::fs::write(directory.path().join("toy_facing_requirements.json"), "{").unwrap();
        assert!(
            load(directory.path())
                .err()
                .unwrap()
                .contains("Parse generated")
        );
        write_inputs(directory.path(), "12.1.5.70391", 1);
        std::fs::write(
            directory.path().join("toy_required_areas.json"),
            r#"{"build":"fixture","spells":{"61810":1103},"groups":{"1103":[]}}"#,
        )
        .unwrap();
        assert!(
            load(directory.path())
                .err()
                .unwrap()
                .contains("area group has no members")
        );
    }
}
