//! A fixed-length DPS row whose SimC reference has `override.bloodlust=0` gets no
//! lust: the follower lust policy does not cast it in a fixed fight.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

/// Bloodlust, Heroism, Time Warp, Fury of the Aspects and Primal Rage.
const LUST: [u32; 5] = [2825, 32182, 80353, 390386, 264667];

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn fixed_fight_without_reference_lust_has_no_lust_on_any_actor() {
    // Arcane casts Time Warp and Elemental casts Heroism or Bloodlust natively.
    for spec in [62, 262] {
        let _seed = crate::random::seed_scope(1);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, spec, 90);
        for _ in 0..5 {
            app.update();
        }
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
        for _ in 0..300 {
            app.update();
        }
        let world = app.world_mut();
        let casts: Vec<u32> = world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess && event.source == Some(actor.to_bits())
            })
            .filter_map(|event| event.spell_id)
            .collect();
        assert!(casts.len() > 5, "spec{spec} is fighting: {casts:?}");
        assert!(
            !casts.iter().any(|spell| LUST.contains(spell)),
            "spec{spec} cast lust: {casts:?}"
        );
        let mut holders = world.query::<(Entity, &crate::auras::ActiveAuras)>();
        for (unit, auras) in holders.iter(world) {
            assert!(
                !auras.iter().any(|aura| LUST.contains(&aura.spell_id)),
                "spec{spec}: {unit:?} holds lust"
            );
        }
    }
}
