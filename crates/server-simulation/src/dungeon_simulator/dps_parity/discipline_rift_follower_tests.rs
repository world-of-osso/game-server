use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn discipline_mind_blast_opens_rift_and_void_blast_heals_atonement() {
    let mut party = super::super::healer_simc_replay::snapshot_party(256, 90, 1);
    let healer = party.healer;
    let enemy = party.sources[0];
    super::super::healer_simc_replay::hold(&mut party, healer);
    for spell in [447444, 450405, 449912, 448403, 449880] {
        assert!(crate::spell_cast::try_cast_triggered(
            party.app.world_mut(),
            healer,
            spell,
            Some(healer),
            1
        ));
    }
    crate::auras::apply_aura(
        party.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: healer,
            target: healer,
            spell_id: 194384,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let mut health = party.app.world_mut().get_mut::<Health>(healer).unwrap();
    health.current = health.max * 0.3;
    crate::spell_cast::creature_cast(party.app.world_mut(), healer, 8092, Some(enemy), false)
        .unwrap();
    for _ in 0..80 {
        party.app.update();
    }
    assert!(crate::auras::has_aura(party.app.world(), healer, 459314));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_action(party.app.world(), healer, 585).unwrap(),
        Some(450215)
    );
    assert_eq!(
        crate::spell_cast::knowledge::resolve_action(party.app.world(), healer, 8092).unwrap(),
        Some(8092)
    );
    let start = party.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::spell_cast::creature_cast(party.app.world_mut(), healer, 450215, Some(enemy), false)
        .unwrap();
    for _ in 0..80 {
        party.app.update();
    }
    let events = &party.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(450215)
                && event.amount > 0),
        "Void Blast damage absent: {events:?}"
    );
    assert!(
        events.iter().any(|event| event.kind == CombatLogKind::Heal
            && event.source == Some(healer.to_bits())
            && event.spell_id == Some(94472)
            && event.amount > event.overflow),
        "Atonement heal absent: {events:?}"
    );
    crate::auras::remove_auras(
        party.app.world_mut(),
        healer,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 459314,
    );
    assert_eq!(
        crate::spell_cast::knowledge::resolve_action(party.app.world(), healer, 585).unwrap(),
        Some(585)
    );
}
