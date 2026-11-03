//! Protection Divine Guidance uses its modified talent value.
use super::*;
use crate::auras::{ActiveAuras, AuraSource};
use crate::spell_cast::{effects::factor_capture::FactorCapture, powers::PowerPools};
use shared::components::PowerType;

#[test]
#[ignore = "requires canonical world database and MID2 Protection inputs"]
fn divine_guidance_uses_protection_modified_talent_value() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(66, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let raw = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(433106)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    let modified =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 433106, 0);
    println!("divine guidance talent raw={raw} modified={modified}");
    assert_eq!(raw, 20.0);
    assert_eq!(modified, 35.0);
    for _ in 0..5 {
        let mut pools = run.app.world_mut().get_mut::<PowerPools>(actor).unwrap();
        let pool = pools
            .pools
            .iter_mut()
            .find(|pool| pool.power == PowerType::HolyPower)
            .unwrap();
        pool.current = pool.max;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
    }
    let stacks = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 433106 && aura.source == AuraSource::Cast)
        .unwrap()
        .stacks;
    assert_eq!(stacks, 5);
    run.app.world_mut().insert_resource(FactorCapture::new(
        actor,
        target,
        433808,
        "divine_guidance",
        1,
    ));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 26573, Some(actor), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let result = run.app.world().resource::<FactorCapture>().export();
    assert_eq!(result["captured_hits"], 1);
    let hit = result["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["spell_id"] == 433808 && event["event"] == "direct_amount")
        .expect("Divine Guidance damage factor capture");
    let factors = &hit["factors"];
    let attack_power = factors["a_power"].as_f64().unwrap();
    let amount = factors["native_scaled_amount"].as_f64().unwrap();
    assert!((amount - attack_power * 17.5).abs() < 0.1, "{factors}");
}
