//! Level-90 replay healers use their tracked profile item instances.

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_and_holy_replay_healers_equip_profile_items() {
    use shared::item_data::ItemDefinitionSource;

    for spec in [256, 257] {
        let run = super::healer_simc_replay::snapshot_party(spec, 90, 1);
        let world = run.app.world();
        let profile = server_follower_apl::profiles::profile(spec).unwrap();
        let items = world
            .get::<crate::stats::EquippedItems>(run.healer)
            .unwrap();
        let modifiers = world
            .get::<crate::stats::EquippedItemModifiers>(run.healer)
            .unwrap();
        assert_eq!(items.items.len(), profile.equipment.len());
        for item in &profile.equipment {
            let slot = crate::followers::parity_profile::slot(&item.slot).unwrap();
            assert!(
                items
                    .items
                    .contains(&(slot, ItemDefinitionSource::Retail, item.item_id)),
                "spec {spec} slot {:?} must contain profile item {}",
                item.slot,
                item.item_id
            );
            assert!(
                modifiers.items.contains(&(slot, item.modifiers())),
                "spec {spec} slot {:?} must use profile instance modifiers",
                item.slot
            );
        }
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_replay_uses_its_equipped_soulcoiler() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    let _seed = crate::random::seed_scope(1);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    let healer = run.healer;
    run.app
        .insert_resource(server_follower_apl::registry::FixedLengthFight { duration: 120.0 });
    super::healer_simc_replay::engage(&mut run);
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..600 {
        run.app.update();
        if run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .any(|event| {
                event.source == Some(healer.to_bits())
                    && event.spell_id == Some(1307578)
                    && event.kind == CombatLogKind::AuraApplied
            })
        {
            break;
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(healer.to_bits())
                && event.spell_id == Some(1307578)
                && event.kind == CombatLogKind::AuraApplied),
        "the real follower must use its equipped Soulcoiler: {events:?}"
    );
}
