//! Ultimate Penitence 421453: actual channel, bolts, Atonement and Pain extension.
use super::test_harness::*;
use crate::auras::ActiveAuras;
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

#[test]
fn ultimate_penitence_channels_bolts_extends_pain_and_transfers_atonement() {
    let mut h = harness(
        Vec::new(),
        &[
            421453, 421434, 421543, 432154, 194384, 81749, 94472, 589, 390686, 390786, 390787,
        ],
        5,
    )
    .unwrap();
    h.app
        .init_resource::<super::test_harness::ObservedCombatEvents>();
    let caster = h.caster;
    let enemy = h.npc;
    *h.app.world_mut().get_mut::<Health>(enemy).unwrap() = Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    };
    let ally = spawn_player(&mut h.app, 5, 1.0);
    h.app.world_mut().entity_mut(caster).insert((
        ActiveSpec(256),
        crate::class_progression::KnownSpells::fixture(vec![421453, 390686, 390786]),
    ));
    h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 10.0;
    for (target, spell) in [(ally, 194384), (enemy, 589)] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster,
                target,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let initial_duration = h
        .app
        .world()
        .get::<ActiveAuras>(enemy)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 589 && aura.caster == caster)
        .unwrap()
        .duration_ms;
    h.cast_and_deliver(421453, Some(enemy));
    h.advance(160);
    let events = &h
        .app
        .world()
        .resource::<super::test_harness::ObservedCombatEvents>()
        .0;
    let damage = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(421543)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(enemy.to_bits())
        })
        .count();
    for (index, _) in events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.spell_id == Some(421543) && event.kind == CombatLogKind::Damage)
    {
        assert!(
            events[index + 1..]
                .iter()
                .take_while(|event| event.kind != CombatLogKind::Damage
                    || event.source != Some(caster.to_bits()))
                .any(|event| event.spell_id == Some(94472)
                    && event.kind == CombatLogKind::Heal
                    && event.target == Some(ally.to_bits())),
            "each bolt transfers Atonement"
        );
    }
    assert_eq!(
        damage,
        25,
        "full native channel must deal 25 bolts: {:?}",
        events
            .iter()
            .filter(|event| matches!(event.spell_id, Some(421453 | 421434 | 421543)))
            .collect::<Vec<_>>()
    );
    let pain = h
        .app
        .world()
        .get::<ActiveAuras>(enemy)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 589 && aura.caster == caster)
        .unwrap();
    assert_eq!(pain.duration_ms, initial_duration + 25 * 1500);
    assert!(
        crate::auras::has_aura(h.app.world(), caster, 390787),
        "Weal and Woe"
    );
}
