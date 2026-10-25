//! Driver dispatch and report contracts without native catalogs or private profiles.
use super::*;

struct RecordingDriver {
    decisions: u32,
    fail: bool,
}

impl SpecDriver for RecordingDriver {
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if self.fail {
            return Err("native decision rejected".into());
        }
        assert_eq!(targets.len(), 2);
        assert!(
            targets
                .iter()
                .all(|&target| world.get_entity(target).is_ok())
        );
        world
            .entity_mut(actor)
            .insert(Name::new("literal decision"));
        self.decisions += 1;
        Ok(())
    }

    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FIRE_76f8118"
    }

    fn report(&self) -> Value {
        json!({"decisions": self.decisions})
    }
}

fn fixture(fail: bool) -> Run {
    let mut app = App::new();
    let actor = app.world_mut().spawn_empty().id();
    let targets = vec![
        app.world_mut().spawn_empty().id(),
        app.world_mut().spawn_empty().id(),
    ];
    Run {
        app,
        actor,
        damage_targets: targets.clone(),
        targets,
        target_waves: vec![],
        profile: serde_json::from_value(json!({
            "spec_id": 63, "level": "90", "race": "human",
            "equipment": [], "consumables": {}
        }))
        .unwrap(),
        gaps: vec![],
        player_rotation: None,
        spec_driver: Some(Box::new(RecordingDriver { decisions: 0, fail })),
    }
}

#[test]
fn selected_driver_receives_actor_targets_and_reports_only_its_state() {
    let mut run = fixture(false);
    drive_player(&mut run).unwrap();
    drive_player(&mut run).unwrap();
    assert_eq!(
        run.app.world().get::<Name>(run.actor).unwrap().as_str(),
        "literal decision"
    );
    let output = driver_report(&run);
    assert_eq!(output["driver"], "LITERAL_SIMC_FIRE_76f8118");
    assert_eq!(output["fire_rotation"], json!({"decisions": 2}));
    for key in [
        "enhancement_rotation",
        "havoc_rotation",
        "arms_rotation",
        "devastation_rotation",
        "frost_mage_rotation",
        "fury_rotation",
        "windwalker_rotation",
        "ret_rotation",
        "marksmanship_rotation",
        "beast_mastery_rotation",
        "shadow_rotation",
    ] {
        assert_eq!(output[key], Value::Null, "inactive report {key}");
    }
    assert!(!output.contains_key("outlaw_rotation"));
    assert!(!output.contains_key("elemental_rotation"));
    assert!(!output.contains_key("assassination_rotation"));
}

#[test]
fn selected_driver_errors_are_not_replaced_by_another_rotation() {
    let mut run = fixture(true);
    assert_eq!(
        drive_player(&mut run).unwrap_err(),
        "native decision rejected"
    );
    assert!(run.app.world().get::<Name>(run.actor).is_none());
    assert_eq!(
        driver_report(&run)["fire_rotation"],
        json!({"decisions": 0})
    );
}

#[test]
fn nonliteral_follower_label_and_null_reports_are_unchanged() {
    let mut run = fixture(false);
    run.spec_driver = None;
    let output = driver_report(&run);
    assert_eq!(output["driver"], "NATIVE_FOLLOWER_KIT");
    assert_eq!(output["fire_rotation"], Value::Null);
}
