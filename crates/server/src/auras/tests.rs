//! Aura runtime tests with real spells from data/world.db.

use shared::casting::CastState;
use shared::components::{
    AuraView, CombatStatus, Health, Mana, Player, Position, UnitAuras, UnitFactionTemplate,
};
use shared::protocol::{CancelAura, CombatChannel, CombatLogEvent, CombatLogKind};

use super::*;
use crate::spell_cast::test_harness::{
    Harness, harness, harness_with_native_enemy_dodge_reduction, varied, world_db,
};

const SHADOW_WORD_PAIN: u32 = 589;
const RENEW: u32 = 139;
const PURIFY: u32 = 527;
const FLASH_HEAL: u32 = 2061;
const SELFLESS_HEALER: u32 = 114250;
/// Warrior passive: PROC_TRIGGER_SPELL 42 on DEAL_MELEE_SWING (mask 0x4),
/// 100% chance, triggers 236282 (AP-scaled damage).
const DEVASTATOR: u32 = 236279;
const DEVASTATOR_DAMAGE: u32 = 236282;
/// Priest passive: ADD_PCT_MODIFIER 108, ChangeCastTime (10) -5% on
/// SpellClassSet 6 mask_0 0xA80, which matches Flash Heal (mask_0 0x800).
const UNWAVERING_WILL: u32 = 373456;
/// Priest passive: ADD_PCT_MODIFIER 108, PowerCost0 (14) -50% on
/// SpellClassSet 6 mask_1 0x81, which matches Purify (mask_1 0x1).
const MENTAL_AGILITY: u32 = 341167;
/// MOD_INCREASE_SPEED 31 +70% on self.
const SPRINT: u32 = 2983;
/// MOD_DECREASE_SPEED 33 -50%.
const HAMSTRING: u32 = 1715;
/// MOD_ROOT_2 455.
const FROST_NOVA: u32 = 122;

const CENARION_GUARD: u32 = 15184;
const AUTHORED_GUARD_AURA: u32 = 18950;

fn cold_guard_harness() -> Harness {
    crate::spell_cast::test_harness::harness_with_plugins(
        Vec::new(),
        &[AUTHORED_GUARD_AURA],
        4,
        false,
        |app| {
            app.add_plugins(crate::grid_activity::GridActivityPlugin);
            app.add_systems(FixedUpdate, crate::zones::apply_addon_auras);
        },
    )
    .expect("authored guard spell data")
}

fn spawn_authored_guard(h: &mut Harness) -> (Entity, Position, crate::maps::InMap) {
    use rand::SeedableRng;
    let data = crate::test_data::world_data();
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| {
            spawn.template_entry == CENARION_GUARD
                && spawn.movement_type == 0
                && data
                    .addon_for(spawn)
                    .is_some_and(|addon| addon.auras.contains(&AUTHORED_GUARD_AURA))
        })
        .expect("actual stationary Cenarion guard with authored addon18950");
    let template = data.template_for(spawn.template_entry, 0).unwrap();
    let map = crate::maps::InMap::continent(u32::from(spawn.map));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    let creature = crate::zones::spawn_creature(
        &mut commands,
        data,
        template,
        &mut crate::zones::CreatureSpawnContext {
            map,
            difficulty_id: crate::zones::map_difficulty_id(h.app.world(), map),
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        spawn,
        &mut rng,
    )
    .expect("authored guard native spawn level context");
    queue.apply(h.app.world_mut());
    let home = *h.app.world().get::<Position>(creature).unwrap();
    assert!(
        h.app
            .world()
            .get::<crate::zones::AddonAuras>(creature)
            .unwrap()
            .0
            .contains(&AUTHORED_GUARD_AURA)
    );
    (creature, home, map)
}

#[test]
fn cold_authored_spawn_defers_addon_until_real_player_wake_in_same_fixed_step() {
    let mut h = cold_guard_harness();
    let (creature, home, map) = spawn_authored_guard(&mut h);
    let far = Position {
        x: home.x + 10_000.0,
        ..home
    };
    h.app.world_mut().entity_mut(h.caster).insert((far, map));
    let health = h.health(creature);
    let population = h
        .app
        .world_mut()
        .query::<&Npc>()
        .iter(h.app.world())
        .count();
    h.advance(1);
    assert!(
        h.app.world().get::<GridDormant>(creature).is_some(),
        "real grid registration must put the cold spawn to sleep"
    );
    assert!(
        auras_of(&h, creature, AUTHORED_GUARD_AURA).is_empty(),
        "inactive grid applied its authored addon effect"
    );
    assert!(
        h.app.world().get::<ActiveAuras>(creature).is_none(),
        "dormant cold population must not initialize aura state"
    );
    assert!(
        h.app
            .world()
            .get::<crate::zones::AddonAuras>(creature)
            .is_some()
    );
    assert_eq!(h.health(creature), health);

    let before = h.app.world().resource::<Time<Fixed>>().elapsed();
    h.app.world_mut().entity_mut(h.caster).insert(home);
    h.advance(1);
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - before,
        std::time::Duration::from_millis(50)
    );
    assert!(h.app.world().get::<GridDormant>(creature).is_none());
    let applied = auras_of(&h, creature, AUTHORED_GUARD_AURA);
    assert_eq!(
        applied.len(),
        1,
        "authored addon must apply in the same fixed step as real player wake"
    );
    assert!(
        h.app
            .world()
            .get::<crate::zones::AddonAuras>(creature)
            .is_none()
    );
    assert_eq!(h.health(creature), health);
    h.advance(1);
    assert_eq!(auras_of(&h, creature, AUTHORED_GUARD_AURA), applied);
    h.app.world_mut().entity_mut(h.caster).insert(far);
    h.advance(1);
    assert!(h.app.world().get::<GridDormant>(creature).is_some());
    h.app.world_mut().entity_mut(h.caster).insert(home);
    h.advance(1);
    assert!(h.app.world().get::<GridDormant>(creature).is_none());
    assert_eq!(
        auras_of(&h, creature, AUTHORED_GUARD_AURA),
        applied,
        "waking again must not reset or reapply the addon"
    );
    assert_eq!(h.health(creature), health);
    assert_eq!(
        h.app
            .world_mut()
            .query::<&Npc>()
            .iter(h.app.world())
            .count(),
        population
    );
}

#[test]
fn active_authored_spawn_applies_its_addon_in_its_first_fixed_step() {
    let mut h = cold_guard_harness();
    let (creature, home, map) = spawn_authored_guard(&mut h);
    h.app.world_mut().entity_mut(h.caster).insert((home, map));
    let health = h.health(creature);
    h.advance(1);
    assert!(h.app.world().get::<GridDormant>(creature).is_none());
    let applied = auras_of(&h, creature, AUTHORED_GUARD_AURA);
    assert_eq!(applied.len(), 1);
    h.advance(1);
    assert_eq!(auras_of(&h, creature, AUTHORED_GUARD_AURA), applied);
    assert_eq!(h.health(creature), health);
}

fn apply(app: &mut App, caster: Entity, target: Entity, spell_id: u32) {
    let spell = app
        .world()
        .resource::<SpellStore>()
        .get(spell_id)
        .cloned()
        .unwrap();
    apply_aura(
        app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: aura_effect_mask(&spell),
            source: AuraSource::Cast,
        },
    );
}

fn auras_of(h: &Harness, unit: Entity, spell_id: u32) -> Vec<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(unit)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| aura.spell_id == spell_id)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn view_of(h: &Harness, unit: Entity, spell_id: u32) -> Option<AuraView> {
    h.app.world().get::<UnitAuras>(unit).and_then(|view| {
        view.auras
            .iter()
            .find(|aura| aura.spell_id == spell_id)
            .cloned()
    })
}

fn log_of(h: &Harness, kind: CombatLogKind, spell_id: u32) -> Vec<CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| event.kind == kind && event.spell_id == Some(spell_id))
        .cloned()
        .collect()
}

fn frames_for(ms: u32) -> usize {
    (ms / 50) as usize
}

/// Retribution Paladin 1258011 effect 0: OVERRIDE_SPELL_POWER_BY_AP_PCT 366, 96%
/// (world.db spell_effect 1266223; specialization_spells 7293, spec 70).
const RETRIBUTION_PASSIVE: u32 = 1258011;
/// Flash of Light: HEAL, BasePoints 0, no SpellScaling, 3.156 × spell power, 5% variance
/// (world.db spell_effect 10981).
const FLASH_OF_LIGHT: u32 = 19750;
const PALADIN: u8 = 2;

/// A human paladin at `level` knowing `spells`, with computed stats and no crit.
fn paladin(spells: &[u32], level: u8) -> Harness {
    let mut h = crate::spell_cast::test_harness::harness_with(Vec::new(), spells, PALADIN, true)
        .expect("world spell data");
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Level(level));
    h.advance(3);
    h
}

fn character_stats(h: &Harness) -> crate::stats::CharacterStats {
    *h.app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
}

/// Flash of Light on self from 1 health; the healed amount.
fn self_flash_of_light(h: &mut Harness) -> i32 {
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 1.0;
    let before = log_of(h, CombatLogKind::Heal, FLASH_OF_LIGHT).len();
    h.cast_and_deliver(FLASH_OF_LIGHT, Some(h.caster));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let heals = log_of(h, CombatLogKind::Heal, FLASH_OF_LIGHT);
    assert_eq!(heals.len(), before + 1);
    let amount = heals[before].amount;
    assert_eq!(h.health(h.caster), 1.0 + amount as f32);
    amount
}

/// The live defect: a level-4 Initial-spec (Strength) paladin's Flash of Light healed 0.
/// Healing power is Intellect for every mana class (Unit::SpellBaseHealingBonusDone,
/// Unit.cpp:7622-7624); damage spell power stays Intellect-primary only (Unit.cpp:7118).
#[test]
fn spellpower_strength_paladin_level_four_flash_of_light_heals_with_intellect() {
    let mut h = paladin(&[FLASH_OF_LIGHT], 4);
    let stats = character_stats(&h);
    // gtClassStats paladin level 4: 22 Str, 26 Int (human +0).
    assert_eq!(
        (stats.primary.strength, stats.primary.intellect),
        (22.0, 26.0)
    );
    assert_eq!(stats.spell_power, 0.0);
    // 3.156 × 26 = 82.056, ±2.5%: 80.0..=84.1.
    let amount = self_flash_of_light(&mut h);
    assert!((80..=84).contains(&amount), "{amount}");
}

/// Retribution's passive makes spell and healing power int32(96% × AP + 0.5)
/// (Unit.cpp:7103-7108, 7602-7607) and follows attack power; unlearning it restores
/// the Intellect rule.
#[test]
fn spellpower_override_retribution_passive_uses_attack_power() {
    use shared::components::UnitStats;

    let mut h = paladin(&[RETRIBUTION_PASSIVE, FLASH_OF_LIGHT], 10);
    let stats = character_stats(&h);
    // Level 10: 33 Str, 1 AP per Str: 96% × 33 + 0.5 = 32.18 -> 32.
    assert_eq!(stats.attack_power, 33.0);
    assert_eq!(stats.spell_power, 32.0);

    let mut mods = *h.app.world().get::<StatModifiers>(h.caster).unwrap();
    mods.primary_flat = UnitStats {
        strength: 67.0,
        ..default()
    };
    h.app.world_mut().entity_mut(h.caster).insert(mods);
    h.advance(1);
    let stats = character_stats(&h);
    assert_eq!((stats.attack_power, stats.spell_power), (100.0, 96.0));
    // 3.156 × 96 = 302.976, ±2.5%: 295.4..=310.6.
    let amount = self_flash_of_light(&mut h);
    assert!((295..=311).contains(&amount), "{amount}");

    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            FLASH_OF_LIGHT,
        ]));
    h.advance(3);
    assert_eq!(character_stats(&h).spell_power, 0.0);
}

/// Paladin passive 137026: effect 6 MOD_BLOCK_PERCENT 51 +7 (world.db spell_effect
/// 260671); effect 7 MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN 417 with SpellClassMask
/// [0x20000000, 0, 0x10000000, 0] (260672).
const PALADIN_PASSIVE: u32 = 137026;
/// Block 107: SPELL_EFFECT_BLOCK 23 (`Player::m_canBlock`).
const BLOCK: u32 = 107;
/// Worn Wooden Shield, INVTYPE_SHIELD.
const WORN_WOODEN_SHIELD: u32 = 2362;

/// A level 10 paladin knowing `spells` with item data loaded, the shield in its off
/// hand when `shield`.
fn paladin_with_shield(spells: &[u32], shield: bool) -> Harness {
    let mut h = paladin(spells, 10);
    let path = crate::test_data::world_db_path();
    let world = h.app.world_mut();
    world.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    world.insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    let items = if shield {
        vec![(
            shared::protocol::EquipmentSlot::OffHand,
            shared::item_data::ItemDefinitionSource::Retail,
            WORN_WOODEN_SHIELD,
        )]
    } else {
        Vec::new()
    };
    world
        .entity_mut(h.caster)
        .insert(crate::stats::EquippedItems { items });
    h.advance(3);
    h
}

/// Swings out of 10 000 rolls a level-matched creature blocks against `defense`.
fn blocked_rolls(defense: crate::melee::PlayerDefense) -> usize {
    use crate::melee::{MeleeOutcome, Swinger, Victim, roll_melee_outcome, swing_chances};
    let chances = swing_chances(
        Swinger::Creature { miss_pct: 5.0 },
        Victim::Player(defense),
        true,
    );
    (0..10_000)
        .filter(|&roll| roll_melee_outcome(&chances, roll) == MeleeOutcome::Block)
        .count()
}

/// Player::UpdateBlockPercentage (StatSystem.cpp:490-507): with `CanBlock` and a
/// shield, block is 5 + Σ MOD_BLOCK_PERCENT + rating; the passive's +7 makes 12%,
/// which the hit table rolls as 1200 of 10 000 swings.
#[test]
fn block_percent_aura_adds_to_shield_block_chance() {
    let h = paladin_with_shield(&[PALADIN_PASSIVE, BLOCK], true);
    let defense = character_stats(&h).defense;
    assert_eq!(defense.block, 12.0);
    assert_eq!(blocked_rolls(defense), 1200);

    let base = character_stats(&paladin_with_shield(&[BLOCK], true)).defense;
    assert_eq!(base.block, 5.0);
    assert_eq!(blocked_rolls(base), 500);
}

/// Without a shield (Unit.cpp:2843-2846) the aura gives no block.
#[test]
fn block_percent_aura_without_a_shield_cannot_block() {
    let h = paladin_with_shield(&[PALADIN_PASSIVE, BLOCK], false);
    let defense = character_stats(&h).defense;
    assert_eq!(defense.block, 0.0);
    assert_eq!(blocked_rolls(defense), 0);
}

/// Divine Toll 375576: StartRecoveryCategory 133, 1500 ms, SPELL_ATTR0_IS_ABILITY,
/// paladin class mask word 2 0x10000000 (affected by the passive's 417).
const DIVINE_TOLL: u32 = 375576;

/// The GCD the paladin starts casting Divine Toll at the kobold with `haste_pct`.
fn divine_toll_gcd_ms(spells: &[u32], haste_pct: f32) -> u32 {
    let mut h = harness(Vec::new(), spells, PALADIN).expect("world spell data");
    h.advance(3);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = haste_pct;
    h.cast_and_deliver(DIVINE_TOLL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let gcds: Vec<u32> = h
        .inbox
        .cooldowns
        .iter()
        .filter(|update| update.is_gcd && update.spell_id == DIVINE_TOLL)
        .map(|update| update.duration_ms)
        .collect();
    assert_eq!(gcds.len(), 1, "{:?}", h.inbox.cooldowns);
    gcds[0]
}

/// Spell::TriggerGlobalCooldown (Spell.cpp:9220-9224): a spell an affecting 417 aura
/// matches has its GCD × ModHasteRegen (1 / (1 + haste)), clamped to 750-1500 ms.
#[test]
fn gcd_by_haste_regen_aura_scales_affected_gcd_with_haste() {
    let with_aura = [PALADIN_PASSIVE, DIVINE_TOLL];
    // 1500 / 1.25 = 1200.
    assert_eq!(divine_toll_gcd_ms(&with_aura, 25.0), 1200);
    // 1500 / 2.5 = 600 -> 750 floor.
    assert_eq!(divine_toll_gcd_ms(&with_aura, 150.0), 750);
    assert_eq!(divine_toll_gcd_ms(&with_aura, 0.0), 1500);
    // Without the aura an IS_ABILITY spell's GCD ignores haste (Spell.cpp:9209-9218).
    assert_eq!(divine_toll_gcd_ms(&[DIVINE_TOLL], 25.0), 1500);
}

#[test]
fn spellpower_override_leaves_intellect_caster_unchanged() {
    let mut h = crate::spell_cast::test_harness::harness_with(Vec::new(), &[FLASH_HEAL], 5, true)
        .expect("world spell data");
    h.advance(3);
    let stats = character_stats(&h);
    assert_eq!(stats.spell_power, stats.primary.intellect);
    assert!(stats.spell_power > 0.0);
}

#[test]
fn shadow_word_pain_ticks_periodic_damage_then_expires() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_WORD_PAIN], 5) else {
        return;
    };
    h.cast_and_deliver(SHADOW_WORD_PAIN, Some(h.npc));
    // Effect 0: 0.168 × SP 100 = 16.8 ± 2.5% direct. Effect 1: PERIODIC_DAMAGE
    // 0.1246 × 100 = 12.46 → 12 every 2 s for 16 s (duration index 387).
    let direct = 5000.0 - h.health(h.npc);
    assert!(varied(16.8).contains(&direct), "{direct}");
    let aura = auras_of(&h, h.npc, SHADOW_WORD_PAIN);
    assert_eq!(aura.len(), 1);
    assert!(aura[0].harmful);
    assert_eq!(aura[0].duration_ms, 16_000);
    h.advance(frames_for(16_000) + 4);
    let ticks: Vec<_> = log_of(&h, CombatLogKind::Damage, SHADOW_WORD_PAIN)
        .into_iter()
        .filter(|event| event.periodic)
        .collect();
    assert_eq!(ticks.len(), 8);
    assert!(ticks.iter().all(|tick| tick.amount == 12));
    assert_eq!(h.health(h.npc), 5000.0 - direct - 8.0 * 12.0);
    assert!(auras_of(&h, h.npc, SHADOW_WORD_PAIN).is_empty());
    assert!(view_of(&h, h.npc, SHADOW_WORD_PAIN).is_none());
    assert_eq!(
        log_of(&h, CombatLogKind::AuraApplied, SHADOW_WORD_PAIN).len(),
        1
    );
    assert_eq!(
        log_of(&h, CombatLogKind::AuraRemoved, SHADOW_WORD_PAIN).len(),
        1
    );
}

#[test]
fn dormant_creature_skips_aura_ticks_then_catches_up_on_reactivation() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_WORD_PAIN], 5) else {
        return;
    };
    h.cast_and_deliver(SHADOW_WORD_PAIN, Some(h.npc));
    let direct = 5000.0 - h.health(h.npc);
    assert!(varied(16.8).contains(&direct), "{direct}");
    let npc = h.npc;
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(crate::grid_activity::GridDormant);
    // 20 s dormant: past all 8 two-second ticks and the 16 s expiry.
    h.advance(frames_for(20_000));
    let periodic = |h: &Harness| {
        log_of(h, CombatLogKind::Damage, SHADOW_WORD_PAIN)
            .into_iter()
            .filter(|event| event.periodic)
            .count()
    };
    assert_eq!(periodic(&h), 0);
    assert_eq!(h.health(h.npc), 5000.0 - direct);
    assert_eq!(auras_of(&h, h.npc, SHADOW_WORD_PAIN).len(), 1);

    h.app
        .world_mut()
        .entity_mut(npc)
        .remove::<crate::grid_activity::GridDormant>();
    h.advance(2);
    assert_eq!(periodic(&h), 8);
    assert_eq!(h.health(h.npc), 5000.0 - direct - 8.0 * 12.0);
    assert!(auras_of(&h, h.npc, SHADOW_WORD_PAIN).is_empty());
    assert!(view_of(&h, h.npc, SHADOW_WORD_PAIN).is_none());
}

/// AuraInterruptFlags EnteringCombat (0x10000000) only.
const SHROUDING: u32 = 28548;

#[test]
fn combat_entered_without_auras_does_not_interrupt_a_later_aura() {
    let Some(mut h) = harness(Vec::new(), &[SHROUDING], 5) else {
        return;
    };
    let npc = h.npc;
    h.app.world_mut().entity_mut(npc).insert(CombatStatus(true));
    h.advance(2);
    apply(&mut h.app, npc, npc, SHROUDING);
    h.advance(2);
    assert_eq!(auras_of(&h, npc, SHROUDING).len(), 1);

    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(CombatStatus(false));
    h.advance(2);
    h.app.world_mut().entity_mut(npc).insert(CombatStatus(true));
    h.advance(1);
    assert!(auras_of(&h, npc, SHROUDING).is_empty());
}

#[test]
fn a_fight_starting_interrupts_an_entering_combat_aura() {
    let Some(mut h) = harness(Vec::new(), &[SHROUDING], 5) else {
        return;
    };
    let (caster, npc) = (h.caster, h.npc);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(CombatStatus(false));
    h.advance(2);
    apply(&mut h.app, caster, caster, SHROUDING);
    h.advance(2);
    assert_eq!(auras_of(&h, caster, SHROUDING).len(), 1);

    crate::combat_refs::set_in_combat_with(h.app.world_mut(), caster, npc);
    h.advance(1);
    assert!(auras_of(&h, caster, SHROUDING).is_empty());
}

#[test]
fn renew_heals_every_tick() {
    let Some(mut h) = harness(Vec::new(), &[RENEW], 5) else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 500.0;
    h.cast_and_deliver(RENEW, None);
    // PERIODIC_HEAL 0.60928 × SP 100 = 60.9 → 61 on apply (Renew has
    // SPELL_ATTR5_EXTRA_INITIAL_PERIOD) and every 3 s for 15 s: 6 ticks
    // (`AuraEffect::GetTotalTicks`, SpellAuraEffects.cpp:945-946).
    h.advance(frames_for(15_000) + 4);
    let ticks: Vec<_> = log_of(&h, CombatLogKind::Heal, RENEW)
        .into_iter()
        .filter(|event| event.periodic)
        .collect();
    assert_eq!(ticks.len(), 6);
    assert!(ticks.iter().all(|tick| tick.amount == 61));
    assert_eq!(h.health(h.caster), 500.0 + 6.0 * 61.0);
}

const RECUPERATE: u32 = 1_231_411;
const RECUPERATE_HEAL: u32 = 1_231_418;

fn recuperate_harness(health: f32) -> Harness {
    let mut h = harness(Vec::new(), &[RECUPERATE, RECUPERATE_HEAL], 5)
        .expect("Recuperate world spell data");
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = health;
    // Keep this proof about the spell, not the independent out-of-combat regen.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<crate::spell_cast::powers::PlayerHealthRegen>();
    h
}

#[test]
fn recuperate_stops_on_the_tick_that_fills_health() {
    let mut h = recuperate_harness(925.0);
    h.cast_and_deliver(RECUPERATE, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(auras_of(&h, h.caster, RECUPERATE_HEAL).len(), 1);
    h.advance(frames_for(1_000));
    assert_eq!(h.health(h.caster), 975.0);
    assert_eq!(auras_of(&h, h.caster, RECUPERATE_HEAL).len(), 1);
    h.advance(frames_for(1_000));
    assert_eq!(h.health(h.caster), 1000.0);
    assert!(auras_of(&h, h.caster, RECUPERATE_HEAL).is_empty());
    h.advance(frames_for(10_000));
    let heals = log_of(&h, CombatLogKind::Heal, RECUPERATE_HEAL);
    assert_eq!(
        heals
            .iter()
            .map(|event| event.amount - event.overflow)
            .collect::<Vec<_>>(),
        vec![50, 25]
    );
    assert_eq!(heals[1].overflow, 25);
    assert_eq!(
        log_of(&h, CombatLogKind::AuraRemoved, RECUPERATE_HEAL).len(),
        1
    );
}

#[test]
fn recuperate_at_full_health_ends_without_heal_logs() {
    let mut h = recuperate_harness(1000.0);
    h.cast_and_deliver(RECUPERATE, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(frames_for(1_000));
    assert!(auras_of(&h, h.caster, RECUPERATE_HEAL).is_empty());
    h.advance(frames_for(10_000));
    assert!(log_of(&h, CombatLogKind::Heal, RECUPERATE_HEAL).is_empty());
}

#[test]
fn recuperate_healing_aura_ends_its_channel_at_full_health() {
    let mut h = recuperate_harness(975.0);
    h.cast_and_deliver(RECUPERATE_HEAL, None);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.advance(frames_for(1_000));
    assert_eq!(h.health(h.caster), 1000.0);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(auras_of(&h, h.caster, RECUPERATE_HEAL).is_empty());
    h.advance(frames_for(10_000));
    let heals = log_of(&h, CombatLogKind::Heal, RECUPERATE_HEAL);
    assert_eq!(heals.len(), 1);
    assert_eq!(heals[0].amount - heals[0].overflow, 25);
}

#[test]
fn recuperate_rule_does_not_stop_renew_at_full_health() {
    let mut h = harness(Vec::new(), &[RENEW], 5).expect("Renew world spell data");
    h.cast_and_deliver(RENEW, None);
    h.advance(frames_for(3_000));
    assert_eq!(auras_of(&h, h.caster, RENEW).len(), 1);
    h.advance(frames_for(12_000) + 4);
    let heals = log_of(&h, CombatLogKind::Heal, RENEW);
    assert_eq!(heals.len(), 6);
    assert!(
        heals
            .iter()
            .all(|event| event.periodic && event.amount == 61 && event.overflow == 61)
    );
}

#[test]
fn stacking_aura_increments_to_max_stacks() {
    let Some(mut h) = harness(Vec::new(), &[SELFLESS_HEALER], 2) else {
        return;
    };
    for _ in 0..6 {
        apply(&mut h.app, h.caster, h.caster, SELFLESS_HEALER);
    }
    let aura = auras_of(&h, h.caster, SELFLESS_HEALER);
    assert_eq!(aura.len(), 1);
    assert_eq!((aura[0].stacks, aura[0].max_stacks), (4, 4));
    h.advance(1);
    assert_eq!(view_of(&h, h.caster, SELFLESS_HEALER).unwrap().stacks, 4);
}

#[test]
fn same_caster_refresh_resets_duration_and_other_casters_coexist() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_WORD_PAIN], 5) else {
        return;
    };
    apply(&mut h.app, h.caster, h.npc, SHADOW_WORD_PAIN);
    let first_expiry = auras_of(&h, h.npc, SHADOW_WORD_PAIN)[0].expires_at.unwrap();
    h.advance(40);
    apply(&mut h.app, h.caster, h.npc, SHADOW_WORD_PAIN);
    let aura = auras_of(&h, h.npc, SHADOW_WORD_PAIN);
    assert_eq!(aura.len(), 1);
    let refreshed = aura[0].expires_at.unwrap();
    assert!(
        (refreshed - first_expiry - 2.0).abs() < 1e-6,
        "{refreshed} vs {first_expiry}"
    );
    h.advance(1);
    assert_eq!(
        log_of(&h, CombatLogKind::AuraRefreshed, SHADOW_WORD_PAIN).len(),
        1
    );

    let other = h
        .app
        .world_mut()
        .spawn((
            Player {
                name: "Other".into(),
                race: 1,
                class: 5,
                appearance: default(),
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    apply(&mut h.app, other, h.npc, SHADOW_WORD_PAIN);
    assert_eq!(auras_of(&h, h.npc, SHADOW_WORD_PAIN).len(), 2);
}

/// Monster template 14 (EnemyGroup includes players): the NPC regards the
/// caster as hostile, so its auras on the caster are harmful. The harness
/// kobold (25) is only hostile from the player side.
fn make_npc_hostile_to_players(h: &mut Harness) {
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(UnitFactionTemplate(14));
}

fn cancel(h: &mut Harness, spell_id: u32) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<CancelAura>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(CancelAura { spell_id });
    h.advance(3);
}

#[test]
fn cancel_aura_removes_own_buff_but_not_a_debuff() {
    let Some(mut h) = harness(Vec::new(), &[RENEW, SHADOW_WORD_PAIN], 5) else {
        return;
    };
    make_npc_hostile_to_players(&mut h);
    apply(&mut h.app, h.caster, h.caster, RENEW);
    apply(&mut h.app, h.npc, h.caster, SHADOW_WORD_PAIN);
    cancel(&mut h, SHADOW_WORD_PAIN);
    cancel(&mut h, RENEW);
    assert!(auras_of(&h, h.caster, RENEW).is_empty());
    assert_eq!(auras_of(&h, h.caster, SHADOW_WORD_PAIN).len(), 1);
    assert!(view_of(&h, h.caster, RENEW).is_none());
    assert!(view_of(&h, h.caster, SHADOW_WORD_PAIN).unwrap().harmful);
}

#[test]
fn purify_dispels_a_magic_debuff_combat_log_metadata() {
    let mut h = harness(Vec::new(), &[PURIFY, SHADOW_WORD_PAIN], 5).expect("spell fixture");
    let before = std::time::SystemTime::now();
    make_npc_hostile_to_players(&mut h);
    apply(&mut h.app, h.npc, h.caster, SHADOW_WORD_PAIN);
    assert_eq!(auras_of(&h, h.caster, SHADOW_WORD_PAIN)[0].dispel_type, 1);
    h.cast_and_deliver(PURIFY, None);
    assert!(auras_of(&h, h.caster, SHADOW_WORD_PAIN).is_empty());
    let dispel: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Dispel)
        .collect();
    assert_eq!(dispel.len(), 1);
    assert_eq!(dispel[0].spell_id, Some(PURIFY));
    assert_eq!(dispel[0].source, Some(h.caster.to_bits()));
    assert_eq!(
        dispel[0].extra_spell_id,
        Some(SHADOW_WORD_PAIN),
        "names the removed aura"
    );
    assert_eq!(dispel[0].target, Some(h.caster.to_bits()));
    crate::spell_cast::test_harness::assert_combat_log_metadata(&h.inbox.log, before);
}

#[test]
fn passive_proc_trigger_fires_on_melee_swing() {
    let Some(mut h) =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[DEVASTATOR, DEVASTATOR_DAMAGE], 1)
    else {
        return;
    };
    h.advance(1);
    let passive = auras_of(&h, h.caster, DEVASTATOR);
    assert_eq!(passive.len(), 1, "known passive auto-applied");
    assert!(passive[0].passive && passive[0].hidden && passive[0].expires_at.is_none());
    let npc = h.npc;
    h.attack(npc);
    // Ten 2 s unarmed swings; missed, dodged and parried ones do not proc.
    h.advance(3 + 9 * 40);
    let landed = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id.is_none() && e.kind == CombatLogKind::Damage)
        .count();
    let procs: Vec<_> = log_of(&h, CombatLogKind::Damage, DEVASTATOR_DAMAGE);
    assert!(landed > 0);
    assert_eq!(procs.len(), landed, "one proc per landed swing");
    for proc in &procs {
        assert_eq!(proc.source, Some(h.caster.to_bits()));
        assert_eq!(proc.target, Some(npc.to_bits()));
        // 236282 effect 0: AP coefficient 1.314 × AP 100, ± 2.5% variance.
        assert!(varied(131.4).contains(&(proc.amount as f32)), "{proc:?}");
    }
}

#[test]
fn pct_modifier_talent_shortens_matched_cast_time() {
    let Some(mut h) = harness(Vec::new(), &[UNWAVERING_WILL, FLASH_HEAL], 5) else {
        return;
    };
    h.advance(1);
    h.cast_and_deliver(FLASH_HEAL, None);
    let cast = h.app.world().get::<CastState>(h.caster).unwrap();
    // Flash Heal base 1500 ms × (1 - 5%).
    assert!((cast.duration - 1.425).abs() < 1e-6, "{}", cast.duration);
}

#[test]
fn pct_modifier_talent_halves_matched_cost() {
    let Some(mut h) = harness(Vec::new(), &[MENTAL_AGILITY, PURIFY], 5) else {
        return;
    };
    h.advance(1);
    let purify = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(PURIFY)
        .cloned()
        .unwrap();
    let costs = crate::spell_cast::spell_costs(h.app.world(), h.caster, &purify).unwrap();
    // Purify: 1.3% of max mana 10000 = 130, halved by Mental Agility.
    assert_eq!(costs.len(), 1);
    assert!((costs[0].1 - 65.0).abs() < 1e-3, "{costs:?}");
    let before = h.app.world().get::<Mana>(h.caster).unwrap().current;
    h.cast_and_deliver(PURIFY, None);
    let after = h.app.world().get::<Mana>(h.caster).unwrap().current;
    assert!(
        before - after > 50.0,
        "cast spent the reduced cost: {before} → {after}"
    );
}

#[test]
fn unit_auras_report_remaining_time_at_last_change() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_WORD_PAIN, RENEW], 5) else {
        return;
    };
    apply(&mut h.app, h.caster, h.npc, SHADOW_WORD_PAIN);
    h.advance(1);
    let first = view_of(&h, h.npc, SHADOW_WORD_PAIN).unwrap();
    assert_eq!(first.duration_ms, 16_000);
    assert!(first.remaining_ms > 15_900 && first.remaining_ms <= 16_000);
    h.advance(20);
    assert_eq!(
        view_of(&h, h.npc, SHADOW_WORD_PAIN).unwrap(),
        first,
        "no resend without a change (ticks do not change the view)"
    );
    apply(&mut h.app, h.npc, h.npc, RENEW);
    h.advance(1);
    let later = view_of(&h, h.npc, SHADOW_WORD_PAIN).unwrap();
    assert!(
        (i64::from(first.remaining_ms) - i64::from(later.remaining_ms) - 1050).abs() <= 50,
        "{} → {}",
        first.remaining_ms,
        later.remaining_ms
    );
    assert_eq!(
        later.flags & AuraView::FLAG_FROM_PLAYER,
        AuraView::FLAG_FROM_PLAYER
    );
}

#[test]
fn unlearned_passive_is_removed() {
    let Some(mut h) = harness(Vec::new(), &[UNWAVERING_WILL], 5) else {
        return;
    };
    h.advance(1);
    assert_eq!(auras_of(&h, h.caster, UNWAVERING_WILL).len(), 1);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    // Queued in Update, synced in the next frame's FixedUpdate.
    h.advance(2);
    assert!(auras_of(&h, h.caster, UNWAVERING_WILL).is_empty());
}

#[test]
fn movement_speed_takes_strongest_increase_and_slow_and_roots_stop() {
    let Some(mut h) = harness(Vec::new(), &[SPRINT, HAMSTRING, FROST_NOVA], 4) else {
        return;
    };
    let speed = |h: &Harness| {
        h.app
            .world()
            .get::<AuraStatMods>(h.caster)
            .unwrap()
            .movement_speed
    };
    apply(&mut h.app, h.caster, h.caster, SPRINT);
    assert!((speed(&h) - 1.7).abs() < 1e-6);
    apply(&mut h.app, h.npc, h.caster, HAMSTRING);
    assert!((speed(&h) - 1.7 * 0.5).abs() < 1e-6);
    apply(&mut h.app, h.npc, h.caster, FROST_NOVA);
    assert_eq!(speed(&h), 0.0);
}

/// Unending Breath 5697: WATER_BREATHING 82 (spell_effect), so `getMaxTimer(BREATH_TIMER)`
/// disables the breath timer.
const UNENDING_BREATH: u32 = 5697;

#[test]
fn water_breathing_aura_marks_the_target_water_breathing() {
    let Some(mut h) = harness(Vec::new(), &[UNENDING_BREATH], 16) else {
        return;
    };
    let mods = |h: &Harness| h.app.world().get::<AuraStatMods>(h.caster).cloned();
    assert!(!mods(&h).unwrap_or_default().water_breathing);
    apply(&mut h.app, h.caster, h.caster, UNENDING_BREATH);
    let mods = mods(&h).unwrap();
    assert!(mods.water_breathing);
    assert_eq!(mods.water_breathing_mult, 1.0);
}

#[test]
fn health_percent_watch_credits_crossings_and_new_auras_but_not_idle_ticks() {
    // Authored 171393: below20% casts171397; above21% casts171404.
    let mut h = harness(Vec::new(), &[171393, 171397, 171404], 5)
        .expect("local health-percent spell fixtures");
    h.advance(1);
    apply(&mut h.app, h.caster, h.caster, 171393);
    h.advance(1);
    assert!(auras_of(&h, h.caster, 171397).is_empty());
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h.advance(1);
    assert_eq!(auras_of(&h, h.caster, 171397).len(), 1);
    remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == 171397,
    );
    h.advance(2);
    assert!(
        auras_of(&h, h.caster, 171397).is_empty(),
        "remaining below the threshold is not another crossing"
    );
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 1000.0;
    h.advance(1);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h.advance(1);
    assert_eq!(auras_of(&h, h.caster, 171397).len(), 1);
    remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| [171393, 171397].contains(&aura.spell_id),
    );
    h.advance(1);
    apply(&mut h.app, h.caster, h.caster, 171393);
    h.advance(1);
    assert_eq!(
        auras_of(&h, h.caster, 171397).len(),
        1,
        "new aura already below threshold fires on application"
    );
}

#[test]
fn stored_spells_carry_labels() {
    let Some(data) = world_db() else {
        return;
    };
    // SpellLabel rows 75/76: labels 16 and 17 on spell 44425.
    let labels = &data.store.get(44425).unwrap().labels;
    assert!(labels.contains(&16) && labels.contains(&17), "{labels:?}");
}

/// Shadow Priest spec passive: HealingAndDamage mods on Mind Blast from
/// class-mask 108 effects (idx 0: -7% mask_2 0x100, idx 11: +360% mask_0
/// 0x2000) and label 218 effect idx 12 (+137%, label 4040).
const SHADOW_PRIEST: u32 = 137033;
const MIND_BLAST: u32 = 8092;

#[test]
fn class_mask_and_label_modifiers_scale_damage() {
    let Some(mut h) = harness(Vec::new(), &[SHADOW_PRIEST, MIND_BLAST], 5) else {
        return;
    };
    h.advance(1);
    h.cast_and_deliver(MIND_BLAST, Some(h.npc));
    h.advance(40);
    let hits = log_of(&h, CombatLogKind::Damage, MIND_BLAST);
    assert_eq!(hits.len(), 1, "{:?}", h.inbox.failed);
    // Active Effect1040380: +360%; 0.78336 × SP100 × 0.93 × 4.6 × 2.37.
    assert!(
        varied(794.2457).contains(&(hits[0].amount as f32)),
        "{:?}",
        hits[0]
    );
}

/// Warrior talent: MOD_MAX_CHARGES 411 +1 and CHARGE_RECOVERY_MOD 453
/// -3000 ms on charge category 1386 (Charge 100: 1 charge, 20 s).
const DOUBLE_TIME: u32 = 103827;
const CHARGE: u32 = 100;

#[test]
fn charge_auras_add_a_charge_and_shorten_recovery() {
    let Some(mut h) = harness(Vec::new(), &[DOUBLE_TIME, CHARGE], 1) else {
        return;
    };
    h.advance(1);
    crate::spell_cast::test_harness::charge_range(&mut h);
    h.cast_and_deliver(CHARGE, Some(h.npc));
    h.advance(2);
    let charges = std::mem::take(&mut h.inbox.charges);
    assert_eq!(
        (charges[0].current, charges[0].max, charges[0].recharge_ms),
        (1, 2, 17_000)
    );
}

/// MOD_STUN 12.
const HAMMER_OF_JUSTICE: u32 = 853;
/// MOD_SILENCE 27.
const SILENCE: u32 = 15487;

#[test]
fn stun_and_silence_block_casting() {
    use shared::spell_data::CastFailReason;
    let Some(mut h) = harness(Vec::new(), &[FLASH_HEAL, HAMMER_OF_JUSTICE, SILENCE], 5) else {
        return;
    };
    apply(&mut h.app, h.npc, h.caster, SILENCE);
    h.cast_and_deliver(FLASH_HEAL, None);
    apply(&mut h.app, h.npc, h.caster, HAMMER_OF_JUSTICE);
    h.cast_and_deliver(FLASH_HEAL, None);
    let reasons: Vec<_> = h.inbox.failed.iter().map(|failed| failed.reason).collect();
    // Flash Heal has PreventionType 1 (silence); the stun is checked first.
    assert_eq!(
        reasons,
        vec![CastFailReason::Silenced, CastFailReason::Stunned]
    );
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
}

/// MOD_TOTAL_STAT_PERCENTAGE 137 +5%, MiscValueB 4 (Stamina).
const POWER_WORD_FORTITUDE: u32 = 21562;
/// MELEE_SLOW 193 +30% (haste).
const BLOODLUST: u32 = 2825;

#[test]
fn stat_auras_fold_into_character_stats_once() {
    let Some(mut h) = crate::spell_cast::test_harness::harness_with(
        Vec::new(),
        &[POWER_WORD_FORTITUDE, BLOODLUST, FLASH_HEAL],
        5,
        true,
    ) else {
        return;
    };
    h.advance(2);
    let stats = |h: &Harness| {
        *h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
    };
    let before = stats(&h);
    apply(&mut h.app, h.caster, h.caster, POWER_WORD_FORTITUDE);
    apply(&mut h.app, h.caster, h.caster, BLOODLUST);
    h.advance(2);
    let after = stats(&h);
    assert!(
        (after.primary.stamina - before.primary.stamina * 1.05).abs() < 1e-3,
        "{before:?} → {after:?}"
    );
    assert_eq!(after.primary.intellect, before.primary.intellect);
    assert!(
        (after.haste_pct - before.haste_pct - 30.0).abs() < 1e-3,
        "{before:?} → {after:?} {:?} {:?}",
        h.app.world().get::<crate::stats::StatModifiers>(h.caster),
        auras_of(&h, h.caster, BLOODLUST)
    );
    h.cast_and_deliver(FLASH_HEAL, None);
    let cast = h.app.world().get::<CastState>(h.caster).unwrap();
    // 1500 ms / (1 + haste%).
    let expected = (1500.0 / (1.0 + after.haste_pct / 100.0)).round() / 1000.0;
    assert!(
        (cast.duration - expected).abs() < 1e-3,
        "{} vs {expected}",
        cast.duration
    );
}
