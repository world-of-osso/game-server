//! Follower white-hit attribution for the fixed-fight parity fixture.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn frost_follower_white_damage_attribution() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 251, 90);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for frame in 0..FIXED_FIGHT_FRAMES {
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current =
            health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        app.update();
        if frame == 100 || frame == 1000 {
            let mods = app
                .world()
                .get::<crate::auras::stat_mods::AuraStatMods>(actor)
                .unwrap();
            let auras = app.world().get::<crate::auras::ActiveAuras>(actor).unwrap();
            for aura in auras
                .iter()
                .filter(|a| matches!(a.spell_id, 137006 | 435005 | 194879))
            {
                println!(
                    "FROST_AURA frame={frame} id={} stacks={} effects={:?}",
                    aura.spell_id,
                    aura.stacks,
                    aura.effects
                        .iter()
                        .map(|e| (e.index, e.aura_type, e.amount))
                        .collect::<Vec<_>>()
                );
            }
            println!(
                "FROST_SWING_MOD frame={frame} auto={}",
                mods.autoattack_damage_mult
            );
        }
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mut count = 0;
    let mut amount = 0;
    for event in events {
        if event.kind == CombatLogKind::Damage
            && event.spell_id.is_none()
            && event.source == Some(actor.to_bits())
        {
            count += 1;
            amount += event.amount;
        }
    }
    println!("FOLLOWER_SWING count={count} damage={amount}");
    assert!(count > 0 && amount > 0);
}
