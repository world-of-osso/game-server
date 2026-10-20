//! Native531/380: SimC76f8118 parse_effects.cpp911-919/1125-1134/1290-1300/1463.
//! Actual Raise Dead/Summon Imp producers; isolated native effect masks are not
//! a full class/loadout certificate. No forged ownership, HP or RNG changes.
use super::test_harness::{Harness, ObservedCombatEvents, harness, world_db};
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::Position;
use shared::protocol::CombatLogKind;

const RAISE_DEAD: u32 = 46585;
const SUMMON_IMP: u32 = 688;
const WOUND: u32 = 209140;
const MUSKET_BARRAGE: u32 = 208227;
const FERAL: u32 = 137011;
const UNHOLY: u32 = 137007;
const DESTRUCTION: u32 = 137046;
const HAUNT: u32 = 48181;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[RAISE_DEAD, SUMMON_IMP, WOUND, MUSKET_BARRAGE],
        9,
    )
    .unwrap();
    let source = world_db().unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &source.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app.insert_resource(source.store.clone());
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(super::guardian::load_guardian_spells(&conn).unwrap());
    h.app
        .insert_resource(crate::hunter_pet::read_pet_catalog(&conn).unwrap());
    h.app
        .insert_resource(crate::pet_control::load_pet_levelup_spells(&conn).unwrap());
    h.app.insert_resource(ObservedCombatEvents::default());
    h.advance(3);
    h
}

fn finish_cast(h: &mut Harness, actor: Entity) {
    let dt = h.app.world().resource::<Time>().delta_secs();
    let frames = h.app.world().get::<CastState>(actor).map_or(0, |cast| {
        ((cast.duration - cast.elapsed).max(0.0) / dt).ceil() as usize
    });
    h.advance(frames + 3);
}

fn guardian(h: &mut Harness, owner: Entity) -> Entity {
    super::creature_cast(h.app.world_mut(), owner, RAISE_DEAD, None, false).unwrap();
    finish_cast(h, owner);
    let world = h.app.world_mut();
    let actor = world
        .query::<(Entity, &super::summon::SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == RAISE_DEAD)
        .unwrap()
        .0;
    let summon = world.get::<super::summon::SummonedCreature>(actor).unwrap();
    assert_eq!(summon.properties_id, 4973);
    assert_eq!((summon.properties.control, summon.properties.title), (1, 2));
    assert_eq!(
        world
            .get::<shared::components::UnitSummonedBy>(actor)
            .unwrap()
            .0,
        owner.to_bits()
    );
    actor
}

fn pet(h: &mut Harness) -> Entity {
    h.advance(32); // Prior native1500ms GCD, not a clock/cooldown edit.
    let owner = h.caster;
    super::creature_cast(h.app.world_mut(), owner, SUMMON_IMP, None, false).unwrap();
    finish_cast(h, owner);
    let actor = crate::hunter_pet::summoned_pet(h.app.world(), owner).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::hunter_pet::HunterPet>(actor)
            .unwrap()
            .owner,
        owner
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(actor)
            .unwrap()
            .template_id,
        416
    );
    actor
}

fn apply_effect(h: &mut Harness, caster: Entity, target: Entity, spell: u32, index: u8) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
}

fn damage(h: &mut Harness, actor: Entity, multiplier: f32) {
    damage_with_spell(h, actor, WOUND, multiplier);
}

fn damage_with_spell(h: &mut Harness, actor: Entity, spell: u32, multiplier: f32) {
    let target = h.npc;
    let before = h.health(target);
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    super::creature_cast(h.app.world_mut(), actor, spell, Some(target), false).unwrap();
    finish_cast(h, actor);
    let hits: Vec<_> = h.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .filter(|event| {
            event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
        })
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "actual native800 spell must land:{hits:?}");
    let hit = &hits[0];
    let crit = if hit.crit { 2.0 } else { 1.0 };
    let source = crate::level_scaling::UnitScaling::of(h.app.world(), actor);
    let victim = crate::level_scaling::UnitScaling::of(h.app.world(), target);
    let expected =
        crate::level_scaling::damage_dealt(&source, &victim, (800.0 * multiplier * crit).round());
    let loss = crate::level_scaling::health_loss(&source, &victim, expected);
    assert_eq!(
        hit.amount, expected as i32,
        "native guardian/pet multiplier:{hit:?}"
    );
    assert_eq!(before - h.health(target), loss);
}

#[test]
fn native_census_guardian_531_positive_native_effect_changes_real_guardian_spell() {
    let mut h = fixture();
    let owner = h.caster;
    let actor = guardian(&mut h, owner);
    damage(&mut h, actor, 1.0);
    apply_effect(&mut h, owner, owner, FERAL, 2); // Actual native531 amount70.
    damage(&mut h, actor, 1.70);
}

#[test]
fn native_census_guardian_531_signed_native_effect_and_removal_use_live_owner() {
    let mut h = fixture();
    let owner = h.caster;
    let actor = guardian(&mut h, owner);
    apply_effect(&mut h, owner, owner, UNHOLY, 3); // Actual native531 amount-22.
    damage(&mut h, actor, 0.78);
    crate::auras::remove_auras(h.app.world_mut(), owner, RemoveReason::Cancelled, |aura| {
        aura.spell_id == UNHOLY
    });
    damage(&mut h, actor, 1.0);
}

#[test]
fn native_census_guardian_531_does_not_modify_actual_controlled_pet_or_owner() {
    let mut h = fixture();
    let actor = pet(&mut h);
    let owner = h.caster;
    apply_effect(&mut h, owner, owner, FERAL, 2);
    damage(&mut h, actor, 1.0);
    damage(&mut h, owner, 1.0);
}

#[test]
fn native_census_guardian_429_531_paired_native_effects_are_not_double_applied() {
    let mut h = fixture();
    let owner = h.caster;
    let ghoul = guardian(&mut h, owner);
    let imp = pet(&mut h);
    // Real paired native5% effects. Other Destruction modifiers are not fabricated.
    apply_effect(&mut h, owner, owner, DESTRUCTION, 2);
    apply_effect(&mut h, owner, owner, DESTRUCTION, 3);
    damage(&mut h, ghoul, 1.05);
    damage(&mut h, imp, 1.05);
}

#[test]
fn native_census_guardian_380_matches_actual_guardian_owner_not_pet_or_player() {
    let mut h = fixture();
    let owner = h.caster;
    let ghoul = guardian(&mut h, owner);
    let imp = pet(&mut h);
    let target = h.npc;
    apply_effect(&mut h, owner, target, HAUNT, 3); // Actual native380 amount16.
    damage_with_spell(&mut h, ghoul, MUSKET_BARRAGE, 1.16);
    damage_with_spell(&mut h, imp, MUSKET_BARRAGE, 1.0);
}

#[test]
fn native_census_guardian_380_does_not_match_foreign_aura_caster_and_removal_ends_bonus() {
    let mut h = fixture();
    let owner = h.caster;
    let ghoul = guardian(&mut h, owner);
    let target = h.npc;
    let other = super::test_harness::spawn_player(&mut h.app, 9, 0.0);
    apply_effect(&mut h, other, target, HAUNT, 3);
    damage_with_spell(&mut h, ghoul, MUSKET_BARRAGE, 1.0);
    crate::auras::remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == HAUNT
    });
    apply_effect(&mut h, owner, target, HAUNT, 3);
    damage_with_spell(&mut h, ghoul, MUSKET_BARRAGE, 1.16);
}

#[test]
fn native_census_guardian_380_preserves_native_ignore_taken_attribute() {
    let mut h = fixture();
    let owner = h.caster;
    let ghoul = guardian(&mut h, owner);
    let target = h.npc;
    assert_ne!(
        super::attributes(world_db().unwrap().store.get(WOUND).unwrap(), 4) & 0x100,
        0
    );
    assert_eq!(
        super::attributes(world_db().unwrap().store.get(MUSKET_BARRAGE).unwrap(), 4) & 0x100,
        0
    );
    apply_effect(&mut h, owner, target, HAUNT, 3);
    damage(&mut h, ghoul, 1.0);
}

#[test]
fn native_census_guardian_531_and_380_change_actual_melee_receipts_without_rng_edits() {
    let mut h = fixture();
    let owner = h.caster;
    let ghoul = guardian(&mut h, owner);
    let target = h.npc;
    assert!(
        crate::level_scaling::UnitScaling::of(h.app.world(), ghoul)
            .swing_damage_range(2.0)
            .is_none(),
        "qualify untuned native ghoul baseline10"
    );
    assert!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(ghoul)
            .is_none()
    );
    let at = *h.app.world().get::<Position>(ghoul).unwrap();
    h.app.world_mut().entity_mut(target).insert(Position {
        x: at.x + 1.0,
        ..at
    });
    apply_effect(&mut h, owner, owner, FERAL, 2);
    apply_effect(&mut h, owner, target, HAUNT, 3);
    // GuardianActor copies its owner's live selection each tick; a manually
    // started swing with no owner selection is correctly stopped by that driver.
    h.select(Some(target));
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..640 {
        // At most16 source2000ms swings at the unchanged50ms fixture step.
        h.advance(1);
        let event = h.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
                    && event.source == Some(ghoul.to_bits())
            });
        if let Some(event) = event {
            let expected = (10.0_f32 * 1.70 * 1.16).round() as i32 * if event.crit { 2 } else { 1 };
            assert_eq!(
                event.amount + event.blocked,
                expected,
                "actual native melee:{event:?}"
            );
            return;
        }
    }
    panic!("no landed native ghoul swing in unchanged32s bound; no RNG or timer overrides");
}
