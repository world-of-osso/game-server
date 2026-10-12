//! Native aura429 ownership and mastery damage contracts; no guessed coefficients.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health, Level};

const COMMAND: u32 = 21_563;
const RESTORATION: u32 = 137_012;
const BEAST_MASTERY: u32 = 76_657;
const FIREBALL: u32 = 133;
const DAMAGE: f32 = 1_000.0;

fn damage_harness(ids: &[u32], class: u8, computed: bool) -> Harness {
    let mut h = harness_with(Vec::new(), ids, class, computed).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.advance(3);
    h
}

fn controlled_pet(h: &mut Harness, owner: Entity) -> Entity {
    let unit = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(crate::hunter_pet::HunterPet { owner, slot: 0 });
    unit
}

fn guardian(h: &mut Harness, owner: Entity) -> Entity {
    let unit = spawn_npc(&mut h.app, 0.0);
    let source = world_db().unwrap();
    let spell = source.store.get(46_585).unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 28)
        .unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &source.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let properties_id = effect.misc_values[1] as u32;
    let properties = super::summon::load_summon_properties(&conn).unwrap().0[&properties_id];
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(super::summon::SummonedCreature {
            owner,
            spell_id: spell.id,
            properties_id,
            properties,
            display_scale: 1.0,
            expires_at: None,
        });
    unit
}

fn spell_loss(h: &mut Harness, caster: Entity, copied: bool) -> f32 {
    let spell = world_db().unwrap().store.get(FIREBALL).unwrap();
    let ctx = super::effects::EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(h.npc);
    if copied {
        super::effects::deal_copied_damage(h.app.world_mut(), &ctx, h.npc, DAMAGE);
    } else {
        super::effects::deal_damage(h.app.world_mut(), &ctx, h.npc, DAMAGE, false, false);
    }
    before - h.health(h.npc)
}

#[test]
fn class_completeness_native_command_buffs_pet_and_guardian_not_owner_stranger_or_copy() {
    let mut h = damage_harness(&[COMMAND, FIREBALL], 3, false);
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .race = 2;
    let owner = h.caster;
    let pet = controlled_pet(&mut h, owner);
    let guardian = guardian(&mut h, owner);
    let stranger = spawn_player(&mut h.app, 3, 0.0);
    let foreign = controlled_pet(&mut h, stranger);
    assert_eq!(spell_loss(&mut h, owner, false), DAMAGE);
    assert_eq!(spell_loss(&mut h, foreign, false), DAMAGE);
    assert_eq!(
        spell_loss(&mut h, pet, true),
        DAMAGE,
        "source copies must not consume owner damage twice"
    );
    let native = world_db().unwrap().store.get(COMMAND).unwrap().effects[0].base_points;
    let expected = (DAMAGE * (1.0 + native / 100.0)).round();
    assert_eq!(spell_loss(&mut h, pet, false), expected);
    assert_eq!(spell_loss(&mut h, guardian, false), expected);
    h.app
        .world_mut()
        .get_mut::<crate::hunter_pet::HunterPet>(pet)
        .unwrap()
        .owner = stranger;
    assert_eq!(
        spell_loss(&mut h, pet, false),
        DAMAGE,
        "ownership changes must not retain the prior bonus"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        owner,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == COMMAND,
    );
    assert_eq!(spell_loss(&mut h, pet, false), DAMAGE);
}

#[test]
fn class_completeness_native_summon_damage_changes_real_guardian_melee() {
    use bevy::ecs::system::RunSystemOnce;
    use shared::protocol::CombatEventType;
    let mut landed_damage = false;
    for _ in 0..32 {
        let mut h = damage_harness(&[RESTORATION, FIREBALL], 11, false);
        let owner = h.caster;
        let guardian = guardian(&mut h, owner);
        h.app
            .world_mut()
            .entity_mut(guardian)
            .remove::<crate::stats::CharacterStats>()
            .insert((
                crate::combat::AutoAttack::new(h.npc),
                crate::auras::AuraStatMods {
                    melee_hit_chance_pct: 100.0,
                    ..default()
                },
            ));
        h.app
            .world_mut()
            .entity_mut(h.npc)
            .insert(shared::components::Position {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            });
        let before = h.health(h.npc);
        h.app
            .world_mut()
            .run_system_once(crate::combat::auto_attack_system)
            .unwrap();
        let spell = world_db().unwrap().store.get(RESTORATION).unwrap();
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.aura == 429)
            .unwrap();
        // Native137012 has no SpellLevels row; its1 RealPointsPerLevel is not zero.
        let level = h.app.world().get::<Level>(owner).unwrap().0;
        let native = effect.base_points + f32::from(level) * effect.real_points_per_level;
        let receipts = h
            .app
            .world()
            .resource::<crate::combat::CombatLog>()
            .range(0.0, f64::MAX);
        let receipt = receipts
            .iter()
            .find(|entry| {
                entry.event.attacker == guardian.to_bits() && entry.event.target == h.npc.to_bits()
            })
            .expect("native guardian must produce a swing receipt");
        let normal = (10.0 * (1.0 + native / 100.0)).round();
        let expected = match &receipt.event.event_type {
            CombatEventType::MeleeDamage => normal,
            CombatEventType::CriticalHit => normal * 2.0,
            CombatEventType::Block => {
                normal - (normal * crate::melee::CREATURE_BLOCKED_PCT / 100.0).trunc()
            }
            CombatEventType::Miss | CombatEventType::Dodge | CombatEventType::Parry => 0.0,
            other => panic!("unexpected guardian swing outcome: {other:?}"),
        };
        assert_eq!(receipt.event.amount, expected, "{receipts:?}");
        assert_eq!(before - h.health(h.npc), expected, "{receipts:?}");
        landed_damage |= expected > 0.0;
    }
    assert!(
        landed_damage,
        "avoidance alone cannot prove the native owner damage modifier"
    );
}

#[test]
fn class_completeness_native_mastery_summon_bonus_tracks_real_owner_stat_changes() {
    const ELEMENTAL_BLAST_MASTERY: u32 = 173_184;
    let mut h = damage_harness(&[BEAST_MASTERY, FIREBALL, ELEMENTAL_BLAST_MASTERY], 3, true);
    let source = world_db().unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &source.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), ActiveSpec(253)));
    h.advance(4);
    let owner = h.caster;
    let pet = controlled_pet(&mut h, owner);
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap();
    let expected = (DAMAGE * (1.0 + stats.mastery_pct / 100.0)).round();
    assert!(expected > DAMAGE);
    assert_eq!(spell_loss(&mut h, pet, false), expected);
    // This native318 child explicitly multiplies its points by a mastery coefficient
    // in its tooltip; no Lycara points/percent conversion is inferred.
    super::cast_triggered(
        h.app.world_mut(),
        owner,
        ELEMENTAL_BLAST_MASTERY,
        Some(owner),
        0,
    );
    h.advance(4);
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap();
    let changed = (DAMAGE * (1.0 + stats.mastery_pct / 100.0)).round();
    assert!(changed > expected);
    assert_eq!(spell_loss(&mut h, pet, false), changed);
    crate::auras::remove_auras(
        h.app.world_mut(),
        owner,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ELEMENTAL_BLAST_MASTERY,
    );
    h.advance(4);
    assert_eq!(spell_loss(&mut h, pet, false), expected);
}

#[test]
fn class_completeness_native_mastery_refresh_preserves_timed_aura_expiry() {
    const SHIFTING_SANDS: u32 = 413_984;
    const ELEMENTAL_BLAST_MASTERY: u32 = 173_184;
    let mut h = damage_harness(
        &[
            BEAST_MASTERY,
            FIREBALL,
            SHIFTING_SANDS,
            ELEMENTAL_BLAST_MASTERY,
        ],
        3,
        true,
    );
    let source = world_db().unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &source.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((Level(80), ActiveSpec(253)));
    h.advance(4);
    let other = spawn_player(&mut h.app, 3, 0.0);
    super::cast_triggered(h.app.world_mut(), owner, SHIFTING_SANDS, Some(owner), 0);
    // Native413984 caps its area selector at one unit; apply the foreign
    // recipient at the aura boundary to test refresh, not target selection.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: owner,
            target: other,
            spell_id: SHIFTING_SANDS,
            effect_mask: 3,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(2);
    let before = h
        .app
        .world()
        .get::<shared::components::UnitAuras>(owner)
        .unwrap()
        .auras
        .iter()
        .find(|aura| aura.spell_id == SHIFTING_SANDS)
        .unwrap()
        .clone();
    let foreign = crate::auras::has_aura(h.app.world(), other, SHIFTING_SANDS);
    assert!(foreign);
    h.advance(60);
    super::cast_triggered(
        h.app.world_mut(),
        owner,
        ELEMENTAL_BLAST_MASTERY,
        Some(owner),
        0,
    );
    h.advance(4);
    let after = h
        .app
        .world()
        .get::<shared::components::UnitAuras>(owner)
        .unwrap()
        .auras
        .iter()
        .find(|aura| aura.spell_id == SHIFTING_SANDS)
        .unwrap()
        .clone();
    assert_eq!(after.duration_ms, before.duration_ms);
    assert_eq!(after.stacks, before.stacks);
    assert!(after.remaining_ms < before.remaining_ms - 2_000);
    h.advance(150);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        owner,
        SHIFTING_SANDS
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        other,
        SHIFTING_SANDS
    ));
    // Type540's gameplay consumer remains separately unimplemented.
}
