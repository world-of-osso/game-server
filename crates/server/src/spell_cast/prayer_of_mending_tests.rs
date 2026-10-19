//! Native Prayer of Mending admission; TC a352b1fa spell_priest.cpp3636–3806.
use super::test_harness::{Harness, harness, spawn_peer_connection, spawn_player, varied};
use crate::auras::has_aura;
use bevy::prelude::Entity;
use shared::casting::CastState;
use shared::components::Position;
use shared::protocol::CombatLogKind;

const PRAYER_OF_MENDING: u32 = 33_076;
const MENDING_WARD: u32 = 41_635;
const MENDING_HEAL: u32 = 33_110;
const MENDING_JUMP: u32 = 155_793;
/// Native800 Fire, no coefficient or variance; leaves the existing1000HP player at200.
const ANNIHILATION: u32 = 209_140;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            PRAYER_OF_MENDING,
            MENDING_WARD,
            MENDING_HEAL,
            MENDING_JUMP,
            ANNIHILATION,
        ],
        5,
    )
    .unwrap()
}

fn ordinary_damage(h: &mut Harness) {
    let target = h.caster;
    ordinary_damage_to(h, target);
}

fn ordinary_damage_to(h: &mut Harness, target: Entity) {
    native_damage_to(h, ANNIHILATION, target);
}

/// Only the authored zero-delay Annihilation and24yd/s Fireball9053 are used.
fn native_damage_to(h: &mut Harness, spell: u32, target: Entity) {
    wait_for_native_spell_ready(h, spell);
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, spell, Some(target), false).unwrap();
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    let cast_frames = h.app.world().get::<CastState>(h.npc).map_or(0, |cast| {
        ((cast.duration - cast.elapsed).max(0.0) / dt).ceil() as usize
    });
    let speed = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .speed;
    // TC AddUnitTarget minimum missile distance5, not a test clock override.
    let flight = if speed > 0.0 {
        unit_distance(h, h.npc, target).max(5.0) / speed
    } else {
        0.0
    };
    h.advance(cast_frames + (flight / dt).ceil() as usize + 3);
}

/// Advance the real clock to the NPC's authored spell/category/GCD readiness.
/// Fireball9053 has native4000ms recovery; no histories or cooldowns are reset.
fn wait_for_native_spell_ready(h: &mut Harness, spell_id: u32) {
    let world = h.app.world();
    let Some(history) = world.get::<super::SpellHistory>(h.npc) else {
        return;
    };
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    let category = spell.categories.as_ref().map_or(0, |row| row.category);
    let now = super::history_now(world);
    let ready = [
        history.spell_ready_at.get(&spell_id),
        history.category_ready_at.get(&category),
        history.gcd_ready_at.get(&super::gcd_category(spell)),
    ]
    .into_iter()
    .flatten()
    .fold(now, |ready, at| ready.max(*at));
    if ready > now {
        let dt = world.resource::<bevy::prelude::Time>().delta_secs_f64();
        h.advance(((ready - now) / dt).ceil() as usize + 1);
    }
}

fn unit_distance(h: &Harness, from: Entity, to: Entity) -> f32 {
    let a = h.app.world().get::<Position>(from).unwrap();
    let b = h.app.world().get::<Position>(to).unwrap();
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn wait_for_native_jump(h: &mut Harness, from: Entity, to: Entity) {
    let speed = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(MENDING_JUMP)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .speed;
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    h.advance((unit_distance(h, from, to).max(5.0) / speed / dt).ceil() as usize + 3);
}

fn owned_ward(h: &Harness, target: Entity, priest: Entity) -> Option<&crate::auras::AuraInstance> {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == MENDING_WARD && aura.caster == priest)
}

#[test]
fn prayer_of_mending_native_unprotected_damage_does_not_heal_or_jump() {
    let mut h = fixture();
    ordinary_damage(&mut h);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(!has_aura(h.app.world(), h.caster, MENDING_WARD));
    assert!(!h.inbox.log.iter().any(|event| {
        event.spell_id == Some(MENDING_HEAL) && event.kind == CombatLogKind::Heal
    }));
}

#[test]
fn prayer_of_mending_native_normal_player_cast_applies_real_ward() {
    let mut h = fixture();
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(h.caster));
    h.advance(3);
    assert!(
        has_aura(h.app.world(), h.caster, MENDING_WARD),
        "ordinary33076 must create real41635; cast failures: {:?}",
        h.inbox.failed
    );
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let ward = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MENDING_WARD)
        .unwrap();
    // Source initial native jump count4 plus the initial recipient.
    assert_eq!(ward.stacks, 5);
    assert_eq!(ward.caster, h.caster);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::AuraApplied
                && event.spell_id == Some(MENDING_WARD)
                && event.amount == 5)
    );
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::AuraApplied
            && event.spell_id == Some(MENDING_WARD)
            && event.source == Some(h.caster.to_bits())
            && event.target == Some(h.caster.to_bits())
    }));
}

#[test]
fn prayer_of_mending_native_damage_heals_then_delayed_jump_keeps_original_priest() {
    let mut h = fixture();
    let holder = spawn_player(&mut h.app, 1, 12.0);
    let healthy_decoy = spawn_player(&mut h.app, 1, 11.0);
    // Real native injury makes the original Priest the unique injured candidate.
    ordinary_damage(&mut h);
    assert_eq!(h.health(h.caster), 200.0);
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(holder));
    h.advance(3);
    assert!(has_aura(h.app.world(), holder, MENDING_WARD));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    ordinary_damage_to(&mut h, holder);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(MENDING_HEAL)
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(holder.to_bits())
        })
        .expect("damage must execute real33110, attributed to the original Priest");
    // Native SP coefficient0.732 and unchanged fixture healing power100.
    let base = if heal.crit { 146.4 } else { 73.2 };
    assert!(varied(base).contains(&(heal.amount as f32)), "{heal:?}");
    assert_eq!(h.health(holder), 200.0 + heal.amount as f32);
    assert!(!has_aura(h.app.world(), holder, MENDING_WARD));
    assert!(
        !has_aura(h.app.world(), h.caster, MENDING_WARD),
        "the native40yd/s jump must not apply before flight"
    );
    assert!(!has_aura(h.app.world(), healthy_decoy, MENDING_WARD));
    let jump = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(MENDING_JUMP)
        .unwrap();
    let speed = jump.misc.as_ref().unwrap().speed;
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    h.advance((12.0 / speed / dt).ceil() as usize + 3);
    let transferred = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MENDING_WARD)
        .expect("real155793 must transfer the ward after its native flight");
    assert_eq!(transferred.caster, h.caster);
    assert_eq!(transferred.stacks, 4);
    assert!(!has_aura(h.app.world(), healthy_decoy, MENDING_WARD));
}

#[test]
fn prayer_of_mending_native_no_stack_override_preserves_agony_floor_and_refresh() {
    for talent in [false, true] {
        let ids = if talent {
            vec![980, 1_260_209]
        } else {
            vec![980]
        };
        let mut h = harness(Vec::new(), &ids, 9).unwrap();
        if talent {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id: 1_260_209,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        h.cast_and_deliver(980, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 980)
            .unwrap();
        let initial = if talent { 3 } else { 1 };
        assert_eq!(aura.stacks, initial);
        let period = aura
            .effects
            .iter()
            .find(|effect| effect.aura_type == 3)
            .unwrap()
            .period_ms;
        let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
        h.advance((period as f32 / 1000.0 / dt).ceil() as usize);
        let stacks = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 980)
            .unwrap()
            .stacks;
        assert_eq!(stacks, initial + 1);
        h.cast_and_deliver(980, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(h.npc)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 980)
                .unwrap()
                .stacks,
            stacks
        );
    }
}

#[test]
fn prayer_of_mending_native_expiry_does_not_heal_on_later_damage() {
    let mut h = fixture();
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(h.caster));
    let duration = owned_ward(&h, h.caster, h.caster).unwrap().duration_ms;
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    h.advance((duration as f32 / 1000.0 / dt).ceil() as usize + 3);
    assert!(!has_aura(h.app.world(), h.caster, MENDING_WARD));
    ordinary_damage(&mut h);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(MENDING_HEAL))
    );
}

#[test]
fn prayer_of_mending_native_exhausts_five_stacks_with_full_health_jump_ally() {
    const FIREBALL: u32 = 9053; // Native7, no coefficient/variance,24yd/s.
    let mut h = harness(
        Vec::new(),
        &[
            PRAYER_OF_MENDING,
            MENDING_WARD,
            MENDING_HEAL,
            MENDING_JUMP,
            FIREBALL,
        ],
        5,
    )
    .unwrap();
    let priest = h.caster;
    let other = spawn_player(&mut h.app, 1, 12.0);
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(priest));
    let mut holder = priest;
    for remaining in (1..=5).rev() {
        assert_eq!(owned_ward(&h, holder, priest).unwrap().stacks, remaining);
        native_damage_to(&mut h, FIREBALL, holder);
        assert_eq!(h.health(holder), 1000.0);
        assert!(!has_aura(h.app.world(), holder, MENDING_WARD));
        let next = if holder == priest { other } else { priest };
        wait_for_native_jump(&mut h, holder, next);
        if remaining > 1 {
            assert_eq!(owned_ward(&h, next, priest).unwrap().stacks, remaining - 1);
        }
        holder = next;
    }
    assert!(!has_aura(h.app.world(), priest, MENDING_WARD));
    assert!(!has_aura(h.app.world(), other, MENDING_WARD));
    let heals = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(MENDING_HEAL))
        .count();
    assert_eq!(heals, 5);
    native_damage_to(&mut h, FIREBALL, priest);
    assert_eq!(h.health(priest), 993.0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(
                |event| event.kind == CombatLogKind::Heal && event.spell_id == Some(MENDING_HEAL)
            )
            .count(),
        heals
    );
}

#[test]
fn prayer_of_mending_native_no_jump_recipient_consumes_current_ward() {
    const FIREBALL: u32 = 9053;
    let mut h = harness(
        Vec::new(),
        &[
            PRAYER_OF_MENDING,
            MENDING_WARD,
            MENDING_HEAL,
            MENDING_JUMP,
            ANNIHILATION,
            FIREBALL,
        ],
        5,
    )
    .unwrap();
    let priest = h.caster;
    // Native root range40, jump radius30:35 is in root range but out of jump range.
    let holder = spawn_player(&mut h.app, 1, 35.0);
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(holder));
    assert!(owned_ward(&h, holder, priest).is_some());
    ordinary_damage_to(&mut h, holder);
    wait_for_native_jump(&mut h, holder, priest);
    assert!(!has_aura(h.app.world(), holder, MENDING_WARD));
    assert!(!has_aura(h.app.world(), priest, MENDING_WARD));
    let before = h.health(holder);
    let heals = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(MENDING_HEAL))
        .count();
    assert_eq!(heals, 1);
    native_damage_to(&mut h, FIREBALL, holder);
    assert_eq!(h.health(holder), before - 7.0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(
                |event| event.kind == CombatLogKind::Heal && event.spell_id == Some(MENDING_HEAL)
            )
            .count(),
        heals
    );
}

#[test]
fn prayer_of_mending_native_foreign_ward_is_not_consumed_or_used_as_jump_owner() {
    let mut h = fixture();
    let priest = h.caster;
    let holder = spawn_player(&mut h.app, 1, 12.0);
    let foreign = spawn_player(&mut h.app, 5, 70.0);
    let foreign_holder = spawn_player(&mut h.app, 1, 72.0);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            PRAYER_OF_MENDING,
        ]));
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(holder));
    // Both roots use ordinary authenticated player requests, not direct aura writes.
    let main_connection = h.connection;
    h.caster = foreign;
    h.connection = connection;
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(foreign_holder));
    h.advance(3);
    h.caster = priest;
    h.connection = main_connection;
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(owned_ward(&h, foreign_holder, foreign).unwrap().stacks, 5);
    ordinary_damage_to(&mut h, holder);
    wait_for_native_jump(&mut h, holder, priest);
    assert_eq!(owned_ward(&h, priest, priest).unwrap().stacks, 4);
    assert_eq!(owned_ward(&h, foreign_holder, foreign).unwrap().stacks, 5);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(MENDING_HEAL)
                && event.source == Some(foreign.to_bits()))
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(MENDING_HEAL)
                && event.source == Some(priest.to_bits())
                && event.target == Some(holder.to_bits()))
    );
}
