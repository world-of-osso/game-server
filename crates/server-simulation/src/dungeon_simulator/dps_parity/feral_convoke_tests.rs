//! Ordinary selected Cat Convoke must dispatch native free child actions.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_convoke_keeps_its_unhasted_channel_lifetime() {
    use shared::casting::{CastState, CastType};
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(391528, actor, Some(target)),
    )
    .unwrap();
    let mut first = None;
    let mut last = None;
    for _ in 0..120 {
        if run
            .app
            .world()
            .get::<CastState>(actor)
            .is_some_and(|cast| cast.spell_id == 391528 && cast.cast_type == CastType::Channel)
        {
            let now = crate::spell_cast::now(run.app.world());
            first.get_or_insert(now);
            last = Some(now);
        }
        run.app.update();
    }
    let observed =
        last.expect("ordinary Convoke started its actual channel") - first.unwrap() + 0.05; // Native fixture advances at50ms.
    assert!(
        (observed - 4.0).abs() <= 0.10,
        "pinned Convoke has a four-second channel despite native haste, observed{observed}s"
    );
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_cat_convoke_deals_child_damage_without_spending_energy_or_form() {
    let _seed = crate::random::seed_scope(103391528);
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 391528)
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    engage(&mut run);
    let energy = |run: &Run| {
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Energy).unwrap()
    };
    for _ in 0..600 {
        if energy(&run).0 >= energy(&run).1 {
            break;
        }
        run.app.update();
    }
    assert_eq!(
        energy(&run).0,
        energy(&run).1,
        "ordinary native Cat regeneration reaches its actual cap"
    );
    let initial = energy(&run).0;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(391528, actor, Some(target)),
    )
    .expect("ordinary selected Convoke channel with preserved hostile primary");
    let mut minimum = initial;
    for _ in 0..180 {
        run.app.update();
        minimum = minimum.min(energy(&run).0);
    }
    let combat = report::combat(run.app.world(), &run.targets, 9.0);
    let child_damage: u64 = combat["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| {
            matches!(
                action["spell_id"].as_u64(),
                Some(22568 | 5221 | 1822 | 155625 | 274838 | 1244079 | 5176)
            )
        })
        .map(|action| action["damage"].as_u64().unwrap())
        .sum();
    assert!(
        child_damage > 0,
        "real Cat Convoke must execute damaging native children, not only an empty channel"
    );
    assert!(
        (initial - minimum).abs() < 0.01,
        "free Convoke children cannot burn or pay the Cat's Energy: {initial}→{minimum}"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 768),
        "Convoke's off-spec children cannot remove native Cat Form"
    );
}
