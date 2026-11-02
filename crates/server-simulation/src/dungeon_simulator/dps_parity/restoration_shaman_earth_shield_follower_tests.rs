use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn earth_shield_heals_damaged_follower_at_native_cooldown() {
    let mut party = super::super::healer_simc_replay::snapshot_party(264, 90, 1);
    let healer = party.healer;
    let holder = party.slots[0];
    let enemy = party.sources[0];
    {
        let mut health = party.app.world_mut().get_mut::<Health>(holder).unwrap();
        health.current = health.max / 2.0;
    }
    crate::spell_cast::creature_cast(party.app.world_mut(), healer, 974, Some(holder), true)
        .unwrap();
    party.app.update();
    assert!(crate::auras::has_aura(party.app.world(), holder, 974));
    let cursor = party.app.world().resource::<ObservedCombatEvents>().0.len();
    super::super::healer_simc_replay::deliver_party_hit(
        party.app.world_mut(),
        enemy,
        holder,
        1000.0,
    );
    for _ in 0..8 {
        party.app.update();
    }
    let count = |party: &super::super::healer_simc_replay::ClassParty| {
        party.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(379)
                    && event.source == Some(healer.to_bits())
                    && event.target == Some(holder.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    assert_eq!(
        count(&party),
        1,
        "damage must trigger the shaman's heal on its follower"
    );
    super::super::healer_simc_replay::deliver_party_hit(
        party.app.world_mut(),
        enemy,
        holder,
        1000.0,
    );
    for _ in 0..8 {
        party.app.update();
    }
    assert_eq!(
        count(&party),
        1,
        "the shield's native cooldown must prevent an immediate second heal"
    );
    for _ in 0..60 {
        party.app.update();
    }
    super::super::healer_simc_replay::deliver_party_hit(
        party.app.world_mut(),
        enemy,
        holder,
        1000.0,
    );
    for _ in 0..8 {
        party.app.update();
    }
    assert_eq!(
        count(&party),
        2,
        "the shield must heal again after its native cooldown"
    );
}
