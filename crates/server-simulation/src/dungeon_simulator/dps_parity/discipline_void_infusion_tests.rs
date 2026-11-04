use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn void_infusion_boosts_atonement_only_with_talent_and_live_rift() {
    for action in [450215, 47540] {
        let mut ratios = Vec::new();
        for (talent, rift) in [(false, true), (true, false), (true, true)] {
            let _seed = crate::random::seed_scope(1);
            let mut party = super::super::healer_simc_replay::snapshot_party(256, 90, 1);
            let healer = party.healer;
            let enemy = party.sources[0];
            super::super::healer_simc_replay::hold(&mut party, healer);
            party.app.world_mut().entity_mut(healer).insert(
                crate::class_progression::KnownSpells::fixture(vec![450215, 47540, 47666]),
            );
            for unit in party.slots {
                crate::auras::remove_auras(
                    party.app.world_mut(),
                    unit,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| {
                        aura.spell_id == 194384
                            || (unit == healer && [450612, 459314].contains(&aura.spell_id))
                    },
                );
            }
            for spell in [194384]
                .into_iter()
                .chain(talent.then_some(450612))
                .chain(rift.then_some(459314))
            {
                crate::auras::apply_aura(
                    party.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: healer,
                        target: healer,
                        spell_id: spell,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            let mut health = party.app.world_mut().get_mut::<Health>(healer).unwrap();
            health.current = health.max * 0.1;
            let mut enemy_health = party.app.world_mut().get_mut::<Health>(enemy).unwrap();
            enemy_health.max = 1_000_000_000.0;
            enemy_health.current = enemy_health.max;
            let start = party.app.world().resource::<ObservedCombatEvents>().0.len();
            let damage_spell = if action == 47540 {
                crate::spell_cast::creature_cast(
                    party.app.world_mut(),
                    healer,
                    action,
                    Some(enemy),
                    false,
                )
                .unwrap();
                47666
            } else {
                crate::spell_cast::cast_triggered_with_effect0_amount(
                    party.app.world_mut(),
                    healer,
                    action,
                    Some(enemy),
                    1,
                    100.0,
                );
                action
            };
            for _ in 0..80 {
                party.app.update();
            }
            let events = &party.app.world().resource::<ObservedCombatEvents>().0[start..];
            let damage: i64 = events
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Damage
                        && event.source == Some(healer.to_bits())
                        && event.spell_id == Some(damage_spell)
                })
                .map(|event| i64::from(event.amount))
                .sum();
            let healing: i64 = events
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Heal
                        && event.source == Some(healer.to_bits())
                        && event.spell_id == Some(94472)
                        && event.target == Some(healer.to_bits())
                })
                .map(|event| i64::from(event.amount))
                .sum();
            assert!(damage > 0 && healing > 0, "action {action}: {events:?}");
            ratios.push(healing as f32 / damage as f32);
        }
        assert!(
            (ratios[1] / ratios[0] - 1.0).abs() < 0.02,
            "without Rift, action {action}: {ratios:?}"
        );
        assert!(
            (ratios[2] / ratios[0] - 1.75).abs() < 0.02,
            "native75% bonus, action {action}: {ratios:?}"
        );
    }
}
