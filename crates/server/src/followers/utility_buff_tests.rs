//! Each learned raid buff must execute on an actual companion party and settle.
use super::{Follower, kit_tests::level32, update_ai};
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Player, Position};
use shared::group::GroupRole;

fn create_party(h: &mut Harness, unit: Entity) {
    let follower = h.app.world().get::<Follower>(unit).unwrap().clone();
    let owner_class = h.app.world().get::<Player>(h.caster).unwrap().class;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(1));
    let member = Follower::member_id(unit);
    let owner = OnlineMember {
        entity: h.caster.to_bits(),
        name: "UtilityOwner".into(),
        class: owner_class,
        level: 32,
        portrait: default(),
        dungeon_difficulty: 1,
        raid_difficulty: default(),
    };
    let view = OnlineView::from([(1, owner), (member, follower.online(unit, 32, 1))]);
    let mut groups = GroupRegistry::default();
    groups
        .create_companion_party(1, GroupRole::Dps, &[(member, follower.profile.role)], &view)
        .unwrap();
    h.app.insert_resource(groups);
}

fn prepare_out_of_combat(h: &mut Harness, unit: Entity) {
    h.stop_attack();
    for actor in [h.caster, unit, h.npc] {
        crate::combat::combat_stop(h.app.world_mut(), actor);
    }
    let owner_position = *h.app.world().get::<Position>(h.caster).unwrap();
    h.app.world_mut().entity_mut(unit).insert(Position {
        x: owner_position.x + 4.0,
        ..owner_position
    });
    create_party(h, unit);
}

fn aura_results(h: &Harness, root: u32) -> Vec<u32> {
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let spell = store.get(root).unwrap();
    let mut results = Vec::new();
    if spell.effects.iter().any(|effect| effect.effect == 6) {
        results.push(root);
    }
    results.extend(spell.effects.iter().filter_map(|effect| {
        let child = store.get(effect.trigger_spell)?;
        (effect.effect == 64 && child.effects.iter().any(|effect| effect.effect == 6))
            .then_some(child.id)
    }));
    assert!(
        !results.is_empty(),
        "native buff{root} must grant an actual aura"
    );
    results
}

fn assert_native_party_buff(entry: u32, spell: u32) {
    let (mut h, unit) = level32(entry);
    prepare_out_of_combat(&mut h, unit);
    let results = aura_results(&h, spell);
    for _ in 0..120 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    for recipient in [h.caster, unit] {
        assert!(
            results
                .iter()
                .any(|aura| crate::auras::has_aura(h.app.world(), recipient, *aura)),
            "entry{entry} spell{spell} must grant a native result aura to party recipient{recipient:?}; casts{:?}",
            h.inbox
                .go
                .iter()
                .filter(|go| go.caster == unit.to_bits())
                .map(|go| go.spell_id)
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.caster == unit.to_bits() && go.spell_id == spell)
            .count(),
        1,
        "maintained native party buff must not be cast repeatedly"
    );
}

#[test]
fn follower_utility_policy_native_party_mark_of_the_wild() {
    assert_native_party_buff(232800, 1126);
}

#[test]
fn follower_utility_policy_native_party_arcane_intellect() {
    assert_native_party_buff(209059, 1459);
}

#[test]
fn follower_utility_policy_native_party_fortitude() {
    assert_native_party_buff(215011, 21562);
}

#[test]
fn follower_utility_policy_native_party_battle_shout() {
    assert_native_party_buff(215517, 6673);
}

#[test]
fn follower_utility_policy_native_party_skyfury() {
    assert_native_party_buff(209053, 462854);
}

#[test]
fn follower_utility_policy_native_party_blessing_of_the_bronze() {
    assert_native_party_buff(215240, 364342);
}
