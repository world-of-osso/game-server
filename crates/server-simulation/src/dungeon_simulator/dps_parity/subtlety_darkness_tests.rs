//! SimC76f8118 sc_rogue.cpp10924–10932/10982–10996: expiring Shadow Blades
//! grants Lingering Darkness, whose Subtlety mod covers finisher Shadow damage only.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::components::PowerType;

const BLADES: u32 = 121471;
const DARKNESS: u32 = 457273;

fn darkness_modifiers(run: &Run, spell: u32) -> Vec<f64> {
    let capture = run.app.world().resource::<FactorCapture>().export();
    capture["modifiers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["spell_id"] == spell && row["source_spell_id"] == DARKNESS)
        .map(|row| row["value"].as_f64().unwrap())
        .collect()
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_darkness_blades_expiry_amplifies_shadow_finisher_not_poison() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    let (actor, target) = (run.actor, run.targets[0]);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: BLADES,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::set_owned_duration(
        run.app.world_mut(),
        actor,
        actor,
        BLADES,
        1
    ));
    for _ in 0..3 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, BLADES));
    let remaining = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == DARKNESS && aura.caster == actor)
        .and_then(|aura| aura.expires_at)
        .expect("expired Shadow Blades must grant timed Lingering Darkness")
        - crate::spell_cast::now(run.app.world());
    assert!(
        (29.5..=30.0).contains(&remaining),
        "Lingering Darkness lasts its native 30s: {remaining}"
    );

    // sc_rogue.cpp4400–4414: Shadowed Finishers needs the caster's Find Weakness.
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 1264521,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    run.app.insert_resource(FactorCapture::new(
        actor,
        target,
        328082,
        "eviscerate_bonus",
        1,
    ));
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap();
        for kind in [PowerType::Energy, PowerType::ComboPoints] {
            let pool = pools.get_mut(kind).unwrap();
            pool.current = pool.max;
        }
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(196819, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        darkness_modifiers(&run, 328082),
        vec![30.0],
        "Shadowed Finishers damage takes Lingering Darkness effect2 once"
    );

    // sc_rogue.cpp11001–11005: poison gains only via Dark Brew effect4,
    // 30/(100+Deeper Daggers 8); Assassination's effects 0/2 would add 1.3x more.
    let poison = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(315585)
        .unwrap()
        .clone();
    let poison_damage = |world: &World| {
        crate::auras::spell_mods::modify(
            world,
            actor,
            &poison,
            crate::auras::SpellModOp::HealingAndDamage,
            100.0,
        )
    };
    let with_darkness = poison_damage(run.app.world());
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == DARKNESS,
    );
    let ratio = with_darkness / poison_damage(run.app.world());
    assert!(
        (ratio - (1.0 + 30.0 / 108.0)).abs() < 1e-4,
        "poison rises only through Dark Brew, never Assassination's effect1: {ratio}"
    );
}
