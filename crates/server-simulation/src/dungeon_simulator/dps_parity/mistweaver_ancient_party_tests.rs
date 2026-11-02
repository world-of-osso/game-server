use crate::spell_cast::test_harness::{ObservedCombatEvents, spawn_npc, spawn_player};
use shared::components::{Health, Position, UnitFactionTemplate, UnitIsPet};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn ancient_teachings_heals_followers_but_not_pets_or_spell_summons() {
    use crate::spell_cast::summon::{SummonProperties, SummonedCreature};

    let mut party = super::super::healer_simc_replay::snapshot_party(270, 90, 1);
    let healer = party.healer;
    let follower = party.tank;
    let enemy = party.sources[0];
    let position = *party.app.world().get::<Position>(healer).unwrap();
    let faction = party
        .app
        .world()
        .get::<UnitFactionTemplate>(healer)
        .unwrap()
        .clone();
    for unit in party.slots {
        let mut health = party.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = health.max;
    }
    party
        .app
        .world_mut()
        .entity_mut(follower)
        .remove::<Position>();
    party.app.world_mut().entity_mut(follower).insert(position);
    party
        .app
        .world_mut()
        .get_mut::<Health>(follower)
        .unwrap()
        .current = 1.0;
    assert!(
        party
            .app
            .world()
            .get::<crate::followers::Follower>(follower)
            .is_some()
    );
    assert!(
        party
            .app
            .world()
            .get::<crate::zones::TempSummon>(follower)
            .is_some()
    );

    let pet = spawn_player(&mut party.app, 3, position.x);
    let summon = spawn_npc(&mut party.app, position.x);
    let map = *party.app.world().get::<crate::maps::InMap>(healer).unwrap();
    for unit in [pet, summon] {
        party.app.world_mut().entity_mut(unit).remove::<Position>();
        party
            .app
            .world_mut()
            .entity_mut(unit)
            .insert((position, faction.clone(), map));
        party
            .app
            .world_mut()
            .get_mut::<Health>(unit)
            .unwrap()
            .current = 1.0;
    }
    party
        .app
        .world_mut()
        .entity_mut(pet)
        .insert(UnitIsPet(true));
    party
        .app
        .world_mut()
        .entity_mut(summon)
        .insert(SummonedCreature {
            owner: healer,
            spell_id: 325197,
            properties_id: 0,
            properties: SummonProperties {
                control: 0,
                faction: 0,
                title: 2,
                slot: 0,
                flags: 0,
            },
            display_scale: 1.0,
            expires_at: None,
        });
    for unit in [follower, pet, summon] {
        assert!(crate::spell_cast::targets::relation_matches(
            party.app.world(),
            healer,
            unit,
            crate::spell_cast::targets::Relation::Ally
        ));
    }
    let heal_spell = party
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(388024)
        .unwrap()
        .clone();
    let radius = crate::spell_cast::targets::radius(&heal_spell.effects[0], 1);
    let center = crate::spell_cast::targets::position(party.app.world(), healer).unwrap();
    let candidates = crate::spell_cast::targets::area_units(
        party.app.world_mut(),
        healer,
        center,
        radius,
        crate::spell_cast::targets::Relation::Ally,
        0,
    );
    for unit in [follower, pet, summon] {
        assert!(
            candidates.contains(&unit),
            "fixture recipient must be in range"
        );
    }
    crate::auras::apply_aura(
        party.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: healer,
            target: healer,
            spell_id: 388023,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(party.app.world_mut(), healer, 100780, Some(enemy), true)
        .unwrap();
    for _ in 0..4 {
        party.app.update();
    }
    let events = &party.app.world().resource::<ObservedCombatEvents>().0;
    let healed = |unit| {
        events.iter().any(|event| {
            event.kind == CombatLogKind::Heal
                && matches!(event.spell_id, Some(388024 | 388025))
                && event.source == Some(healer.to_bits())
                && event.target == Some(unit)
        })
    };
    assert!(
        healed(follower.to_bits()),
        "Ancient Teachings must heal the follower: {events:?}"
    );
    assert!(
        !healed(pet.to_bits()),
        "Ancient Teachings must not heal pets"
    );
    assert!(
        !healed(summon.to_bits()),
        "Ancient Teachings must not heal spell summons"
    );
}
