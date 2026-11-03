use super::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires MID2 Protection profile, set gear and world data"]
fn dps_parity_protection_free_revenge_earns_next_shield_slam_bonus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(73, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::item_proc_data::has_equipped_bonus(run.app.world(), actor, 1296647).unwrap());
    engage(&mut run);
    auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 5302,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 6572, Some(target), false)
        .unwrap();
    assert!(
        auras::has_aura(run.app.world(), actor, 1300681),
        "free Revenge earns Vengeful Shield"
    );
    for _ in 0..40 {
        run.app.update();
    }
    run.app
        .insert_resource(FactorCapture::new(actor, target, 23922, "shield_slam", 1));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 23922, Some(target), false)
        .unwrap();
    let capture = run.app.world().resource::<FactorCapture>().export();
    let direct = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["spell_id"] == 23922 && event["event"] == "direct_amount")
        .unwrap();
    assert!(
        direct["applied_modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|modifier| modifier["source_spell_id"] == 1300681 && modifier["value"] == 20.0),
        "Shield Slam receives the native 20% set bonus: {direct}"
    );
    assert!(
        !auras::has_aura(run.app.world(), actor, 1300681),
        "Shield Slam consumes the bonus"
    );
    for _ in 0..40 {
        run.app.update();
    }
    auras::remove_auras(
        run.app.world_mut(),
        actor,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == 1296647,
    );
    auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 5302,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 6572, Some(target), false)
        .unwrap();
    assert!(
        !auras::has_aura(run.app.world(), actor, 1300681),
        "without the set aura, free Revenge cannot earn the bonus"
    );
}
