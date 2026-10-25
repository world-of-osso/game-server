//! Observe real native casts and aura/field lifecycles; never dispatch AT hooks manually.
use super::azure_rune_fields::{assert_native_source, fixture};
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;
use std::collections::HashSet;

const RUNE_AURA: u32 = 388065;
const SHIELD: u32 = 390201;
const INACTIVE: u32 = 390238;

#[derive(Clone)]
struct RuneCastEvent {
    caster: Entity,
    spell: u32,
    inside: HashSet<Entity>,
    protected: HashSet<Entity>,
}

#[derive(Resource, Default)]
struct RuneCastEvents(Vec<RuneCastEvent>);

/// OnSpellCast is the engine's public gameplay-script output, including full triggers.
/// This listener observes casts; it does not supply any Rune AT implementation.
struct RuneCastObserver;

impl CreatureAi for RuneCastObserver {
    fn on_spell_cast(&mut self, ctx: &mut Ctx, spell: u32) {
        let caster = ctx.me;
        let inside: HashSet<Entity> = ctx
            .world
            .query::<&SpellAreaTrigger>()
            .iter(ctx.world)
            .filter(|field| field.caster == caster && field.spell_id == RUNE_AURA)
            .flat_map(|field| field.inside.iter().copied())
            .collect();
        let protected = inside
            .iter()
            .copied()
            .filter(|unit| {
                ctx.world
                    .get::<crate::auras::ActiveAuras>(*unit)
                    .is_some_and(|auras| {
                        auras
                            .iter()
                            .any(|aura| aura.spell_id == 388072 && aura.caster == caster)
                    })
            })
            .collect();
        ctx.world
            .resource_mut::<RuneCastEvents>()
            .0
            .push(RuneCastEvent {
                caster,
                spell,
                inside,
                protected,
            });
    }
}

fn observe_rune(h: &mut Harness, rune: Entity) {
    h.app.world_mut().init_resource::<RuneCastEvents>();
    crate::scripted_ai::tests::install_fixture_ai(
        h.app.world_mut(),
        rune,
        Box::new(RuneCastObserver),
    );
}

fn casts(h: &Harness, rune: Entity, spell: u32) -> Vec<RuneCastEvent> {
    h.app
        .world()
        .resource::<RuneCastEvents>()
        .0
        .iter()
        .filter(|event| event.caster == rune && event.spell == spell)
        .cloned()
        .collect()
}

fn rune_position(h: &Harness, rune: Entity, offset: f32) -> Position {
    let mut position = *h.app.world().get::<Position>(rune).unwrap();
    position.x += offset;
    position
}

fn place_player(h: &mut Harness, player: Entity, rune: Entity, offset: f32) {
    let position = rune_position(h, rune, offset);
    let map = *h.app.world().get::<InMap>(rune).unwrap();
    h.app.world_mut().entity_mut(player).insert((position, map));
}

fn observed_fixture(difficulty: u32) -> (Harness, Entity) {
    let (mut h, rune) = fixture(difficulty);
    assert_native_source(&h);
    observe_rune(&mut h, rune);
    for player in [h.players[0].0, h.players[1].0] {
        let position = rune_position(&h, rune, 20.0);
        // Preserve each player's original map until a test explicitly places it.
        h.app.world_mut().entity_mut(player).insert(position);
    }
    (h, rune)
}

fn cast_rune_aura(h: &mut Harness, rune: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), rune, RUNE_AURA, Some(rune), true)
        .expect("actual Rune casts source388065");
    h.advance(0.05);
    assert!(h.has_aura(rune, RUNE_AURA), "native aura prerequisite");
}

fn owned_fields(h: &mut Harness, rune: Entity) -> Vec<Entity> {
    h.app
        .world_mut()
        .query::<(Entity, &SpellAreaTrigger)>()
        .iter(h.app.world())
        .filter(|(_, field)| field.caster == rune && field.spell_id == RUNE_AURA)
        .map(|(entity, _)| entity)
        .collect()
}

fn remove_rune_aura(h: &mut Harness, rune: Entity) {
    let instance = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(rune)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == RUNE_AURA)
        .unwrap()
        .instance_id;
    crate::auras::remove_instance(
        h.app.world_mut(),
        rune,
        instance,
        crate::auras::RemoveReason::Cancelled,
    );
    h.advance(0.05);
}

#[test]
fn azure_rune_native_player_entries_publish_membership_and_cast_shield_once() {
    let (mut h, rune) = observed_fixture(205);
    cast_rune_aura(&mut h, rune);
    let first = h.players[0].0;
    let second = h.players[1].0;
    place_player(&mut h, first, rune, 0.5);
    place_player(&mut h, second, rune, 1.0);
    h.advance(0.05);
    assert!(
        h.has_aura(rune, SHIELD),
        "eligible player entry must cast390201 on its Rune"
    );
    assert!(
        !h.has_aura(first, SHIELD),
        "source cast is Rune self-aura, not a fabricated player buff"
    );
    let events = casts(&h, rune, SHIELD);
    assert_eq!(
        events.len(),
        1,
        "second entering player must not recast an existing shield"
    );
    assert_eq!(
        events[0].inside,
        HashSet::from([first, second]),
        "full new membership is visible during the first actual shield cast"
    );
    place_player(&mut h, second, rune, 20.0);
    h.advance(0.05);
    assert!(h.has_aura(rune, SHIELD));
    place_player(&mut h, second, rune, 1.0);
    h.advance(0.05);
    assert_eq!(
        casts(&h, rune, SHIELD).len(),
        1,
        "later entry does not refresh/recast390201"
    );
}

#[test]
fn azure_rune_native_template_action_precedes_player_callback_and_undoes_on_exit() {
    let (mut h, rune) = observed_fixture(205);
    cast_rune_aura(&mut h, rune);
    let player = h.players[0].0;
    place_player(&mut h, player, rune, 0.5);
    h.advance(0.05);
    assert!(
        h.has_aura(player, 388072),
        "native CAST388072/FRIEND action must run"
    );
    let events = casts(&h, rune, SHIELD);
    assert_eq!(events.len(), 1);
    assert!(
        events[0].protected.contains(&player),
        "DoActions precedes OnUnitEnter"
    );
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(player)
        .unwrap();
    for school in [1, 2, 4, 8, 16, 32, 64] {
        assert_eq!(mods.damage_taken(school), 0.5, "native aura87 -50/mask127");
    }
    place_player(&mut h, player, rune, 20.0);
    h.advance(0.05);
    assert!(
        !h.has_aura(player, 388072),
        "exit undoes native action aura"
    );
    assert!(!h.has_aura(rune, SHIELD));
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(player)
            .unwrap()
            .damage_taken(64),
        1.0
    );
}

#[test]
fn azure_rune_native_nonfriend_npc_is_inside_without_action_or_player_activation() {
    let (mut h, rune) = observed_fixture(205);
    cast_rune_aura(&mut h, rune);
    let npc = h.creatures_of(186737)[0];
    let position = rune_position(&h, rune, 0.5);
    h.app.world_mut().entity_mut(npc).insert(position);
    assert!(
        !crate::spell_cast::targets::relation_matches(
            h.app.world(),
            rune,
            npc,
            crate::spell_cast::targets::Relation::Ally
        ),
        "native Telash is not Rune FRIEND"
    );
    h.advance(0.05);
    let fields = owned_fields(&mut h, rune);
    assert_eq!(fields.len(), 1, "native field prerequisite");
    assert!(
        h.app
            .world()
            .get::<SpellAreaTrigger>(fields[0])
            .unwrap()
            .inside
            .contains(&npc),
        "base membership is not Player-only or FRIEND-filtered"
    );
    assert!(
        !h.has_aura(npc, 388072),
        "FRIEND action must not cast on nonfriend member"
    );
    assert!(
        !h.has_aura(rune, SHIELD),
        "NPC membership cannot activate390201"
    );
    let player = h.players[0].0;
    place_player(&mut h, player, rune, 0.5);
    h.advance(0.05);
    assert_eq!(
        casts(&h, rune, SHIELD)[0].inside,
        HashSet::from([npc, player])
    );
}

#[test]
fn azure_rune_native_same_tick_swap_keeps_shield_until_last_living_exit() {
    let (mut h, rune) = observed_fixture(205);
    cast_rune_aura(&mut h, rune);
    let first = h.players[0].0;
    let second = h.players[1].0;
    place_player(&mut h, first, rune, 0.5);
    h.advance(0.05);
    assert!(h.has_aura(rune, SHIELD), "actual entry prerequisite");
    place_player(&mut h, first, rune, 20.0);
    place_player(&mut h, second, rune, 0.5);
    h.advance(0.05);
    assert!(
        h.has_aura(rune, SHIELD),
        "simultaneous player swap must not drop the Rune shield"
    );
    assert_eq!(
        casts(&h, rune, SHIELD).len(),
        1,
        "no remove/recast flicker across the swap"
    );
    place_player(&mut h, second, rune, 20.0);
    h.advance(0.05);
    assert!(
        !h.has_aura(rune, SHIELD),
        "last living player exit removes390201"
    );
}

#[test]
fn azure_rune_native_npc_other_copy_dead_and_ghost_do_not_activate_shield() {
    let (mut h, rune) = observed_fixture(205);
    cast_rune_aura(&mut h, rune);
    let other_player = h.players[1].0;
    let position = rune_position(&h, rune, 0.5);
    h.app.world_mut().entity_mut(other_player).insert(position);
    let other_rune = h
        .creatures_of(197204)
        .into_iter()
        .find(|unit| *unit != rune)
        .unwrap();
    h.app.world_mut().entity_mut(other_rune).insert(position);
    h.advance(0.05);
    assert!(
        !h.has_aura(rune, SHIELD),
        "native NPC or player in another copy does not activate the Player-only Rune callback"
    );
    for state in [
        shared::death::DeathState::Dead,
        shared::death::DeathState::Ghost,
    ] {
        place_player(&mut h, other_player, rune, 0.5);
        // Typed life-state eligibility only; preserve existing native HP/resources.
        h.app
            .world_mut()
            .entity_mut(other_player)
            .insert(crate::death::PlayerDeathState {
                state,
                ..Default::default()
            });
        h.advance(0.05);
        assert!(
            !h.has_aura(rune, SHIELD),
            "source ActionSetFlags0 excludes dead/ghost players"
        );
    }
    assert_eq!(casts(&h, rune, SHIELD).len(), 0);
}

#[test]
fn azure_rune_native_removal_uses_original_membership_and_only_mythic_modes_deactivate() {
    for difficulty in [1, 2, 205, 23, 8] {
        let (mut h, rune) = observed_fixture(difficulty);
        cast_rune_aura(&mut h, rune);
        let player = h.players[0].0;
        place_player(&mut h, player, rune, 0.5);
        h.advance(0.05);
        assert!(
            h.has_aura(rune, SHIELD),
            "native entry prerequisite at difficulty{difficulty}"
        );
        assert!(
            h.has_aura(player, 388072),
            "real template action prerequisite"
        );
        let fields = owned_fields(&mut h, rune);
        assert_eq!(fields.len(), 1, "actual owned field prerequisite");
        remove_rune_aura(&mut h, rune);
        assert!(h.app.world().get_entity(fields[0]).is_err());
        assert!(
            !h.has_aura(player, 388072),
            "removal undoes template action"
        );
        assert!(
            !h.has_aura(rune, SHIELD),
            "removal exits players after OnRemove"
        );
        let mythic = matches!(difficulty, 23 | 8);
        assert_eq!(
            h.has_aura(rune, INACTIVE),
            mythic,
            "source inactive mode gate for{difficulty}"
        );
        let events = casts(&h, rune, INACTIVE);
        assert_eq!(events.len(), usize::from(mythic));
        if mythic {
            assert_eq!(
                events[0].inside,
                HashSet::from([player]),
                "OnRemove runs before membership cleanup/despawn"
            );
        }
    }
}

#[test]
fn azure_rune_native_mythic_empty_dead_and_ghost_removal_does_not_deactivate() {
    for state in [
        None,
        Some(shared::death::DeathState::Dead),
        Some(shared::death::DeathState::Ghost),
    ] {
        let (mut h, rune) = observed_fixture(23);
        cast_rune_aura(&mut h, rune);
        if let Some(state) = state {
            let player = h.players[1].0;
            place_player(&mut h, player, rune, 0.5);
            h.app
                .world_mut()
                .entity_mut(player)
                .insert(crate::death::PlayerDeathState {
                    state,
                    ..Default::default()
                });
            h.advance(0.05);
        }
        remove_rune_aura(&mut h, rune);
        assert!(
            !h.has_aura(rune, INACTIVE),
            "no living inside player means no inactive cast"
        );
        assert_eq!(casts(&h, rune, INACTIVE).len(), 0);
    }
}

#[test]
fn azure_rune_native_fields_keep_owner_isolation_and_survive_missing_caster_cleanup() {
    let (mut h, first) = observed_fixture(23);
    let second = h
        .creatures_of(197204)
        .into_iter()
        .find(|unit| *unit != first)
        .unwrap();
    observe_rune(&mut h, second);
    cast_rune_aura(&mut h, first);
    cast_rune_aura(&mut h, second);
    let first_player = h.players[0].0;
    let second_player = h.players[1].0;
    place_player(&mut h, first_player, first, 0.5);
    place_player(&mut h, second_player, second, 0.5);
    h.advance(0.05);
    assert!(h.has_aura(first, SHIELD));
    assert!(h.has_aura(second, SHIELD));
    assert!(h.has_aura(first_player, 388072));
    assert!(h.has_aura(second_player, 388072));
    let second_fields = owned_fields(&mut h, second);
    assert_eq!(second_fields.len(), 1);
    remove_rune_aura(&mut h, first);
    assert!(h.has_aura(first, INACTIVE));
    assert!(!h.has_aura(first_player, 388072));
    assert!(
        h.has_aura(second_player, 388072),
        "another Rune's action survives"
    );
    assert!(h.has_aura(second, RUNE_AURA));
    assert!(h.has_aura(second, SHIELD));
    assert!(!h.has_aura(second, INACTIVE));
    assert!(h.app.world().get_entity(second_fields[0]).is_ok());
    h.app.world_mut().entity_mut(second).despawn();
    h.advance(0.05);
    assert!(h.app.world().get_entity(second_fields[0]).is_err());
    assert!(
        !h.has_aura(second_player, 388072),
        "missing caster retires its action"
    );
    assert_eq!(
        casts(&h, second, INACTIVE).len(),
        0,
        "departed caster cannot cast on another Rune"
    );
    assert_eq!(casts(&h, first, INACTIVE).len(), 1);
}
