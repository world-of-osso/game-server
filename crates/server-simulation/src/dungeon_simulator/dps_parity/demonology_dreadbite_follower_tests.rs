//! Actual Dreadbite reads Dreadlash and its Dreadstalker's own Ferocity crit.
use super::follower_dps_diagnostic::*;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

const DREADLASH: u32 = 264078;
const DREADBITE: u32 = 271971;
const FEROCITY: u32 = 453704;

struct Bite {
    mult: f64,
    crit_pct: f64,
    expected_crit_pct: f64,
}

fn first_dreadbite(dreadlash: bool) -> Bite {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>()
        .insert(crate::networking::CombatTarget(Some(target)));
    assert!(
        crate::auras::has_aura(app.world(), actor, DREADLASH),
        "selected Dreadlash"
    );
    if !dreadlash {
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == DREADLASH,
        );
    }
    let health = app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .into_inner();
    health.current = 1_000_000_000.0;
    health.max = health.current;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 193331, Some(actor), true)
        .expect("native Dreadstalker summon");
    app.update();
    let dog = app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(app.world())
        .find(|(_, summon)| summon.owner == actor && summon.spell_id == 193331)
        .map(|(unit, _)| unit)
        .expect("actual Dreadstalker");
    app.insert_resource(FactorCapture::new(dog, target, DREADBITE, "dreadbite", 1));
    for _ in 0..200 {
        app.update();
        let capture = app.world().resource::<FactorCapture>().export();
        let events = capture["events"].as_array().unwrap();
        let Some(result) = events
            .iter()
            .find(|event| event["event"] == "result" && event["spell_id"] == DREADBITE)
        else {
            continue;
        };
        let direct = events
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == DREADBITE)
            .expect("actual Dreadbite input calculation");
        let owner_crit = app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct;
        let ferocity = app
            .world()
            .get::<crate::auras::ActiveAuras>(dog)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == FEROCITY)
            .expect("Flametouched Ferocity on the actual Dreadstalker")
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .amount;
        return Bite {
            mult: direct["factors"]["mult"].as_f64().unwrap(),
            crit_pct: result["factors"]["crit"].as_f64().unwrap() * 100.0,
            expected_crit_pct: f64::from(owner_crit + ferocity),
        };
    }
    panic!("actual Dreadstalker must land Dreadbite within ten seconds");
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_dreadbite_applies_dreadlash_and_ferocity_crit() {
    let with = first_dreadbite(true);
    let without = first_dreadbite(false);
    // SimC76f8118 pets.cpp1270–1288: owner crit plus the dog's own Ferocity crit.
    for bite in [&with, &without] {
        assert!(
            (bite.crit_pct - bite.expected_crit_pct).abs() < 0.01,
            "Dreadbite crit {} expected {}",
            bite.crit_pct,
            bite.expected_crit_pct
        );
    }
    // SimC76f8118 pets.cpp1074–1081: Dreadlash Dummy0 base points 10.
    let ratio = with.mult / without.mult;
    assert!(
        (ratio - 1.10).abs() < 0.001,
        "Dreadlash Dreadbite multiplier ratio {ratio}"
    );
}
