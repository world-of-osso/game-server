//! Aura type registry, keyed by AuraType ID (TrinityCore `SpellAuraDefines.h`).
//! Unhandled types are still applied and replicated; `apply_aura` logs them
//! once per spell.

/// How a handled aura type takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraKind {
    /// Ticks every `EffectAuraPeriod` (`periodic.rs`).
    Periodic,
    /// Folded into `stats::StatModifiers` or `AuraStatMods` (`stat_mods.rs`).
    StatMod,
    /// Spell modifier read by the cast pipeline (`spell_mods.rs`).
    SpellMod,
    /// Casts `EffectTriggerSpell` on proc (`procs.rs`).
    ProcTrigger,
    /// Consumed by incoming damage (`absorb_damage`).
    Absorb,
    /// Native per-effect area damage reduction, restricted by school mask.
    AreaAvoidance,
    /// Redirects native school-mask damage to the aura caster.
    DamageTransfer,
    /// Retaliates after damaging melee swings, not ordinary aura procs.
    DamageShield,
    /// Negates damage of its school mask.
    SchoolImmunity,
    /// Rejects matching spell/effect mechanics and removes matching auras.
    MechanicImmunity,
    /// Rejects SpellEffects IDs in aura37 MiscValue.
    EffectImmunity,
    /// Damage taken from the aura's caster (`damage_taken_multiplier`).
    DamageFromCaster,
    ///Creature victim selection and caster-specific threat generation.
    Taunt,
    /// The caster's preferred victim while applied (`fixate.rs`).
    Fixate,
    /// Behavior lives in `AuraScripts` keyed by spell ID.
    Script,
    /// Casts its trigger spell on a health-percent crossing (`health_pct_trigger.rs`).
    HealthTrigger,
    /// Charge-category max/recovery modifier (`charge_mods`).
    ChargeMod,
    /// Blocks casting (`cast_blocker`); MOD_STUN also roots.
    CastBlock,
    /// Unit state reconciled on every aura change (`unit_state.rs`).
    UnitState,
    /// Client-owned effect selected by replicated aura spell data; no server action.
    ClientVisual,
    /// Chance to reflect a spell back to its caster (`reflect_chance`).
    Reflect,
    /// Read by incoming hit selection (`spell_cast::hit`, `combat`).
    HitDefense,
    /// Read by cast selection and included in aura replication.
    Override,
    /// School/power-mask percentage read by SpellInfo::CalcPowerCost.
    PowerCost,
    /// Permission read by PvP talent selection, including arena preparation.
    TalentSwap,
    /// Trigger a summon on application and remove it with its aura.
    LinkedSummon,
    /// Source-defined area trigger owned by this exact aura effect.
    AreaTrigger,
    /// Trigger and remove an owned child aura; only zero parent amounts are supported.
    Linked,
    /// Permission to keep masked casts/channels moving (`spell_cast`).
    CastMovement,
    /// Masked permission to ignore cast-start form restrictions (`spell_cast::shapeshift`).
    IgnoreShapeshift,
    /// Player casting restriction with native class-family exceptions.
    CastRestriction,
    /// Receiver-side finite harmful aura duration (`mechanic_duration`).
    MechanicDuration,
    /// Target detection-distance modifier read by `creature_aggro::AggroDistance`.
    DetectedRange,
}

pub const PERIODIC_DAMAGE: u32 = 3;
pub const DUMMY: u32 = 4;
pub const MOD_CONFUSE: u32 = 5;
pub const MOD_FEAR: u32 = 7;
pub const PERIODIC_HEAL: u32 = 8;
/// SpellEffect amplitude supplies the fraction of effective damage returned as health.
pub const PERIODIC_LEECH: u32 = 53;
/// Read at threat-generation time by creature_aggro::threat_multiplier.
pub const MOD_THREAT: u32 = 10;
pub const MOD_TAUNT: u32 = 11;
pub const MOD_STUN: u32 = 12;
pub const DAMAGE_SHIELD: u32 = 15;
pub const MOD_STEALTH: u32 = 16;
pub const MOD_STEALTH_DETECT: u32 = 17;
pub const MOD_STEALTH_LEVEL: u32 = 154;
/// Added to the target's creature detection distance before the native minimum clamp.
pub const MOD_DETECTED_RANGE: u32 = 152;
pub const DETECT_STEALTH: u32 = 228;
/// TC a352b1fa SpellInfo.cpp4140: matching optional resource cost only.
pub const MOD_ADDITIONAL_POWER_COST: u32 = 63;
pub const MOD_POWER_COST_SCHOOL_PCT: u32 = 72;
pub const ALLOW_TALENT_SWAPPING: u32 = 158;
pub const ARENA_PREPARATION: u32 = 215;
pub const MOD_INVISIBILITY: u32 = 18;
pub const MOD_INVISIBILITY_DETECT: u32 = 19;
pub const DETECT_VISIBLE_WHILE_INVISIBLE: u32 = 335;
pub const OBS_MOD_HEALTH: u32 = 20;
pub const OBS_MOD_POWER: u32 = 21;
pub const PERIODIC_TRIGGER_SPELL: u32 = 23;
pub const PERIODIC_ENERGIZE: u32 = 24;
pub const MOD_PACIFY: u32 = 25;
pub const MOD_ROOT: u32 = 26;
pub const MOD_SILENCE: u32 = 27;
pub const REFLECT_SPELLS: u32 = 28;
pub const MOD_STAT: u32 = 29;
pub const MOD_INCREASE_SPEED: u32 = 31;
pub const MOD_INCREASE_MOUNTED_SPEED: u32 = 32;
pub const MOD_INCREASE_HEALTH: u32 = 34;
pub const MOD_INCREASE_ENERGY: u32 = 35;
pub const MOD_SHAPESHIFT: u32 = 36;
pub const EFFECT_IMMUNITY: u32 = 37;
pub const MOD_DECREASE_SPEED: u32 = 33;
/// SPELL_AURA_MOD_DODGE_PERCENT: nondiminishing percentage points.
pub const MOD_DODGE_PERCENT: u32 = 49;
pub const MOD_PARRY_PERCENT: u32 = 47;
pub const MECHANIC_IMMUNITY: u32 = 77;
/// Matching class-mask protection read by SpellInfo::CanBeInterrupted.
pub const PREVENT_INTERRUPT: u32 = 203;
/// Interrupter-side permission for the exact spell ID in MiscValue0.
pub const ALLOW_INTERRUPT_SPELL: u32 = 484;
/// MiscValue0 is a creature_immunities row ID, not the mechanics bitmask itself.
pub const MECHANIC_IMMUNITY_MASK: u32 = 147;
pub const MOD_ATTACKER_MELEE_HIT_CHANCE: u32 = 184;
pub const MOD_ATTACKER_RANGED_HIT_CHANCE: u32 = 185;
pub const MOD_ATTACKER_SPELL_HIT_CHANCE: u32 = 186;
/// TC HandleModRating applies integer points to every selected CombatRating.
pub const MOD_RATING: u32 = 189;
/// The three versatility ratings share one value in our CombatRatings component.
pub const VERSATILITY_RATING_MASK: i32 = 0x7000_0000;
/// TC Unit::GetUnitCriticalChanceTaken: victim-owned, non-ranged attacks only.
pub const MOD_ATTACKER_MELEE_CRIT_CHANCE: u32 = 187;
/// TC Unit.cpp2906–2909 / Unit.h804,809: victim current health >= MiscValueB threshold.
pub const MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH: u32 = 183;
/// TC Unit.cpp2924: victim-owned, included before the received-crit zero clamp.
pub const MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE: u32 = 197;
/// TC Unit.cpp2911–2914: victim aura caster must equal the actual attacker.
pub const MOD_CRIT_CHANCE_FOR_CASTER: u32 = 306;
/// TC Unit.cpp2916–2921: actual TempSummon direct summoner owns this victim modifier.
pub const MOD_CRIT_CHANCE_FOR_CASTER_PET: u32 = 339;
/// TrinityCore a352b1fa SpellAuraDefines.h: 287 deflects, 288 ignores facing.
pub const DEFLECT_SPELLS: u32 = 287;
pub const IGNORE_HIT_DIRECTION: u32 = 288;
/// SPELL_AURA_MOD_BLOCK_PERCENT (SpellAuraDefines.h:138, TrinityCore a352b1fa).
pub const MOD_BLOCK_PERCENT: u32 = 51;
/// TC Unit.cpp2889: NPC autoattack source crit; player weapon masks remain unsupported.
pub const MOD_WEAPON_CRIT_PERCENT: u32 = 52;
pub const MOD_HIT_CHANCE: u32 = 54;
pub const MOUNTED: u32 = 78;
/// Caster-owned vehicle boarding aura; runtime support is vehicle-context-specific.
pub const CONTROL_VEHICLE: u32 = 236;
pub const TRANSFORM: u32 = 56;
pub const SCHOOL_IMMUNITY: u32 = 39;
pub const PROC_TRIGGER_SPELL: u32 = 42;
pub const MOD_PACIFY_SILENCE: u32 = 60;
/// TC a352b1fa Unit::RecalculateObjectScale: sum percentage points, not products.
pub const MOD_SCALE: u32 = 61;
pub const MOD_CASTING_SPEED_NOT_STACK: u32 = 65;
pub const SCHOOL_ABSORB: u32 = 69;
pub const REFLECT_SPELLS_SCHOOL: u32 = 74;
pub const MOD_DAMAGE_PERCENT_DONE: u32 = 79;
/// TC HandleModPercentStat: Player-only create/base percentage, stat index or -1.
pub const MOD_PERCENT_STAT: u32 = 80;
/// TC Unit.cpp:6962/8110: percentage done versus creature-type mask, not school.
pub const MOD_DAMAGE_DONE_VERSUS: u32 = 168;
pub const WATER_BREATHING: u32 = 82;
pub const MOD_REGEN: u32 = 84;
pub const MOD_POWER_REGEN: u32 = 85;
pub const MOD_DAMAGE_PERCENT_TAKEN: u32 = 87;
pub const MOD_HEALTH_REGEN_PERCENT: u32 = 88;
pub const PERIODIC_DAMAGE_PERCENT: u32 = 89;
pub const MOD_UNATTACKABLE: u32 = 93;
pub const FEATHER_FALL: u32 = 105;
pub const ADD_FLAT_MODIFIER: u32 = 107;
pub const ADD_PCT_MODIFIER: u32 = 108;
pub const MOD_POWER_REGEN_PERCENT: u32 = 110;
pub const MOD_REGEN_DURING_COMBAT: u32 = 116;
pub const MOD_HEALING_PCT: u32 = 118;
pub const MOD_SPEED_ALWAYS: u32 = 129;
pub const MOD_MOUNTED_SPEED_ALWAYS: u32 = 130;
/// TC Unit.cpp8816: strongest positive mounted bonus, not stacked with aura130.
pub const MOD_MOUNTED_SPEED_NOT_STACK: u32 = 172;
pub const MOD_INCREASE_HEALTH_PERCENT: u32 = 133;
pub const MOD_HEALING_DONE_PERCENT: u32 = 136;
pub const REDUCE_PUSHBACK: u32 = 149;
/// TC Unit::IsWithinMeleeRangeAt adds the attacker's total native modifier.
pub const MOD_AUTOATTACK_RANGE: u32 = 153;
pub const MOD_WATER_BREATHING: u32 = 155;
pub const MOD_HEALTH_REGEN_IN_COMBAT: u32 = 161;
pub const MOD_TOTAL_STAT_PERCENTAGE: u32 = 137;
pub const MOD_MELEE_HASTE: u32 = 138;
pub const MOD_RANGED_HASTE: u32 = 140;
pub const USE_NORMAL_MOVEMENT_SPEED: u32 = 191;
pub const MOD_MELEE_RANGED_HASTE: u32 = 192;
pub const MELEE_SLOW: u32 = 193;
pub const LEARN_SPELL: u32 = 195;
pub const FLY: u32 = 201;
pub const MOD_INCREASE_MOUNTED_FLIGHT_SPEED: u32 = 207;
pub const MOD_MOUNTED_FLIGHT_SPEED_ALWAYS: u32 = 208;
/// TC Unit.cpp8854: strongest positive flight bonus.
pub const MOD_FLIGHT_SPEED_NOT_STACK: u32 = 211;
pub const ADV_FLYING: u32 = 446;
/// Unit::SpellCriticalDamageBonus / CalculateMeleeDamage, school-mask multiplier.
pub const MOD_CRIT_DAMAGE_BONUS: u32 = 163;
/// Flat percentage points, outside combat-rating diminishing returns.
pub const MOD_VERSATILITY: u32 = 471;
/// TC a352b1fa independent melee/ranged AP percent modifiers.
pub const MOD_ATTACK_POWER_PCT: u32 = 166;
pub const MOD_RANGED_ATTACK_POWER_PCT: u32 = 167;
pub const MOD_RAGE_FROM_DAMAGE_DEALT: u32 = 213;
pub const HASTE_SPELLS: u32 = 216;
pub const ADD_PCT_MODIFIER_BY_SPELL_LABEL: u32 = 218;
pub const ADD_FLAT_MODIFIER_BY_SPELL_LABEL: u32 = 219;
pub const PERIODIC_DUMMY: u32 = 226;
pub const MOD_AOE_DAMAGE_AVOIDANCE: u32 = 229;
pub const PROC_TRIGGER_SPELL_WITH_VALUE: u32 = 231;
/// TC a352b1fa HandleUnused: viewer-local humanoid appearance, client-side only.
pub const CHANGE_MODEL_FOR_ALL_HUMANOIDS: u32 = 233;
pub const MOD_SPEED_SLOW_ALL: u32 = 252;
/// Melee attack time only (`HandleModMeleeSpeedPct`); no cast-time effect.
pub const MOD_MELEE_HASTE_3: u32 = 319;
/// TC a352b1fa: HandleNoImmediateEffect; MiscValue0 is ScreenEffect.db2 ID.
pub const SCREEN_EFFECT: u32 = 260;
/// TC a352b1fa SpellAuraEffects.cpp:340; Player::UpdateArmor uses live stats.
pub const MOD_ARMOR_PCT_FROM_STAT: u32 = 268;
/// TC a352b1fa Unit.cpp7056/8178: victim aura owner and incoming school mask.
pub const MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER: u32 = 270;
pub const MOD_SPELL_DAMAGE_FROM_CASTER: u32 = 271;
/// TC a352b1fa Unit::SpellDamageBonusTaken: owner + SpellLabel match.
pub const MOD_SPELL_DAMAGE_FROM_CASTER_BY_LABEL: u32 = 537;
pub const MOD_HEALING_RECEIVED: u32 = 283;
pub const MOD_CRIT_PCT: u32 = 290;
pub const OVERRIDE_SPELLS: u32 = 293;
pub const MASTERY: u32 = 318;
/// TC a352b1fa HandleAuraModNoActions: PreventionType NO_ACTIONS (4).
pub const MOD_NO_ACTIONS: u32 = 321;
pub const ANIM_REPLACEMENT_SET: u32 = 312;
pub const OVERRIDE_ACTIONBAR_SPELLS: u32 = 332;
pub const MOD_MELEE_RANGED_HASTE_2: u32 = 342;
/// TC a352b1fa Unit.cpp2387: signed autoattack-only critical chance.
pub const MOD_AUTOATTACK_CRIT_CHANCE: u32 = 334;
pub const MOD_AUTOATTACK_DAMAGE: u32 = 344;
/// SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT (SpellAuraDefines.h, TrinityCore a352b1fa).
pub const OVERRIDE_SPELL_POWER_BY_AP_PCT: u32 = 366;
pub const MOD_MANA_REGEN_PCT: u32 = 379;
/// TC StatSystem.cpp863-869: MiscValue is a power bitmask, independent of amount.
pub const PREVENT_REGENERATE_POWER: u32 = 294;
/// SPELL_AURA_MOD_CHARGE_RECOVERY_RATE (TC a352b1fa SpellHistory.cpp1003-1005).
pub const MOD_CHARGE_RECOVERY_RATE: u32 = 148;
pub const MOD_MAX_CHARGES: u32 = 411;
pub const CHARGE_RECOVERY_MOD: u32 = 453;
pub const CHARGE_RECOVERY_MULTIPLIER: u32 = 454;
/// TC a352b1fa SpellHistory::GetChargeRecoveryTime: category-scoped ModHasteRegen.
pub const CHARGE_RECOVERY_AFFECTED_BY_HASTE_REGEN: u32 = 457;
pub const MOD_ROOT_2: u32 = 455;
/// SPELL_AURA_MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN (SpellAuraDefines.h:504, TrinityCore a352b1fa).
pub const MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN: u32 = 417;
/// TC a352b1fa SpellHistory::StartCooldown: spell-mask-scoped haste regen.
pub const MOD_COOLDOWN_BY_HASTE_REGEN: u32 = 416;
/// SpellHistory::StartCooldown adds signed milliseconds to matching categories.
pub const MOD_SPELL_CATEGORY_COOLDOWN: u32 = 341;
/// Percent faster/slower cooldown recovery, matched by the native class mask.
pub const MOD_RECOVERY_RATE: u32 = 286;
/// Percent cooldown recovery selected by either native SpellLabel misc value.
pub const MOD_RECOVERY_RATE_BY_SPELL_LABEL: u32 = 143;
pub const MOD_MAX_POWER: u32 = 418;
pub const SET_POWER_POINT_CHARGE: u32 = 494;
pub const LINKED: u32 = 284;
/// SPELL_AURA_MOD_FIXATE (SpellAuraDefines.h, TrinityCore a352b1fa).
pub const MOD_FIXATE: u32 = 128;
pub const LINKED_2: u32 = 285;
pub const LINKED_SUMMON: u32 = 428;
pub const MOD_SUMMON_DAMAGE: u32 = 429;
pub const MOD_BONUS_ARMOR_PCT: u32 = 466;
pub const MOD_STAT_BONUS_PCT: u32 = 467;
pub const MOD_BASE_RESISTANCE_PCT: u32 = 142;
/// Physical bit: armor TOTAL_PCT (StatSystem.cpp:273); other schools unmodelled.
pub const MOD_RESISTANCE_PCT: u32 = 101;
pub const MOD_MINIMUM_SPEED: u32 = 305;
pub const CAST_WHILE_WALKING: u32 = 330;
pub const MOD_IGNORE_SHAPESHIFT: u32 = 275;
pub const DISABLE_CASTING_EXCEPT_ABILITIES: u32 = 263;
pub const DISABLE_ATTACKING_EXCEPT_ABILITIES: u32 = 264;
pub const MECHANIC_DURATION_MOD: u32 = 232;
pub const MECHANIC_DURATION_MOD_NOT_STACK: u32 = 234;
pub const OVERRIDE_SUMMONED_OBJECT: u32 = 258;
/// Read at landing by `networking::fall_damage_multiplier` (`Player::HandleFall`).
pub const MODIFY_FALL_DAMAGE_PCT: u32 = 374;
/// TC HandleForceMoveForward sets UnitFlags2::FORCE_MOVEMENT.
pub const FORCE_MOVE_FORWARD: u32 = 241;
/// TC HandleCreateAreaTrigger: original caster, attached aura target, exact effect lifetime.
pub const CREATE_AREATRIGGER: u32 = 395;

pub fn kind(aura_type: u32) -> Option<AuraKind> {
    use AuraKind::*;
    Some(match aura_type {
        PERIODIC_DAMAGE
        | PERIODIC_LEECH
        | PERIODIC_HEAL
        | OBS_MOD_HEALTH
        | OBS_MOD_POWER
        | PERIODIC_TRIGGER_SPELL
        | PERIODIC_ENERGIZE
        | PERIODIC_DAMAGE_PERCENT => Periodic,
        PERIODIC_DUMMY | DUMMY => Script,
        super::health_pct_trigger::TRIGGER_SPELL_ON_HEALTH_PCT => HealthTrigger,
        MOD_TAUNT => Taunt,
        MOD_FIXATE => Fixate,
        DAMAGE_SHIELD => DamageShield,
        MOD_AOE_DAMAGE_AVOIDANCE => AreaAvoidance,
        super::damage_transfer::SPLIT_DAMAGE_PCT => DamageTransfer,
        SCREEN_EFFECT | CHANGE_MODEL_FOR_ALL_HUMANOIDS => ClientVisual,
        LINKED_SUMMON => LinkedSummon,
        CREATE_AREATRIGGER => AreaTrigger,
        LINKED | LINKED_2 => Linked,
        CAST_WHILE_WALKING => CastMovement,
        MOD_IGNORE_SHAPESHIFT => IgnoreShapeshift,
        DISABLE_CASTING_EXCEPT_ABILITIES
        | DISABLE_ATTACKING_EXCEPT_ABILITIES
        | PREVENT_INTERRUPT
        | ALLOW_INTERRUPT_SPELL => CastRestriction,
        MECHANIC_DURATION_MOD | MECHANIC_DURATION_MOD_NOT_STACK => MechanicDuration,
        MOD_DETECTED_RANGE => DetectedRange,
        ANIM_REPLACEMENT_SET | OVERRIDE_ACTIONBAR_SPELLS | OVERRIDE_SPELLS => Override,
        MOD_STUN | MOD_PACIFY | MOD_SILENCE | MOD_PACIFY_SILENCE | MOD_NO_ACTIONS => CastBlock,
        MOD_ROOT
        | MOD_ROOT_2
        | MOD_STAT
        | MOD_PERCENT_STAT
        | MOD_ARMOR_PCT_FROM_STAT
        | MOD_INCREASE_SPEED
        | MOD_DECREASE_SPEED
        | MOD_CASTING_SPEED_NOT_STACK
        | MOD_DAMAGE_PERCENT_DONE
        | MOD_CRIT_DAMAGE_BONUS
        | MOD_THREAT
        | MODIFY_FALL_DAMAGE_PCT
        | MOD_DAMAGE_PERCENT_TAKEN
        | MOD_POWER_REGEN_PERCENT
        | MOD_HEALING_PCT
        | MOD_SPEED_ALWAYS
        | USE_NORMAL_MOVEMENT_SPEED
        | REDUCE_PUSHBACK
        | MOD_INCREASE_HEALTH
        | MOD_INCREASE_HEALTH_PERCENT
        | MOD_HEALING_DONE_PERCENT
        | MOD_TOTAL_STAT_PERCENTAGE
        | MOD_MELEE_HASTE
        | MOD_RANGED_HASTE
        | MOD_MELEE_RANGED_HASTE
        | MELEE_SLOW
        | HASTE_SPELLS
        | MOD_HEALING_RECEIVED
        | MOD_CRIT_PCT
        | MOD_WEAPON_CRIT_PERCENT
        | MOD_RATING
        | MASTERY
        | MOD_STAT_BONUS_PCT
        | MOD_BONUS_ARMOR_PCT
        | MOD_BASE_RESISTANCE_PCT
        | MOD_RESISTANCE_PCT
        | MOD_MINIMUM_SPEED
        | MOD_MELEE_RANGED_HASTE_2
        | MOD_AUTOATTACK_CRIT_CHANCE
        | MOD_AUTOATTACK_DAMAGE
        | MOD_AUTOATTACK_RANGE
        | MOD_SUMMON_DAMAGE
        | MOD_RAGE_FROM_DAMAGE_DEALT
        | MOD_MANA_REGEN_PCT
        | MOD_HIT_CHANCE
        | MOD_SPEED_SLOW_ALL
        | MOD_MELEE_HASTE_3
        | WATER_BREATHING
        | MOD_WATER_BREATHING
        | MOD_INCREASE_ENERGY
        | MOD_MAX_POWER
        | SET_POWER_POINT_CHARGE
        | MOD_REGEN
        | MOD_HEALTH_REGEN_PERCENT
        | MOD_REGEN_DURING_COMBAT
        | MOD_HEALTH_REGEN_IN_COMBAT
        | MOD_INCREASE_MOUNTED_SPEED
        | MOD_MOUNTED_SPEED_ALWAYS
        | MOD_MOUNTED_SPEED_NOT_STACK
        | MOD_MOUNTED_FLIGHT_SPEED_ALWAYS
        | MOD_FLIGHT_SPEED_NOT_STACK
        | MOD_DODGE_PERCENT
        | MOD_PARRY_PERCENT
        | MOD_BLOCK_PERCENT
        | OVERRIDE_SPELL_POWER_BY_AP_PCT
        | MOD_VERSATILITY
        | MOD_ATTACK_POWER_PCT
        | MOD_RANGED_ATTACK_POWER_PCT
        | MOD_DAMAGE_DONE_VERSUS => StatMod,
        MOD_INVISIBILITY
        | MOD_STEALTH
        | MOD_STEALTH_LEVEL
        | MOD_STEALTH_DETECT
        | DETECT_STEALTH
        | ARENA_PREPARATION
        | MOD_INVISIBILITY_DETECT
        | DETECT_VISIBLE_WHILE_INVISIBLE
        | MOD_SCALE
        | FEATHER_FALL
        | FORCE_MOVE_FORWARD
        | MOD_UNATTACKABLE
        | MOD_CONFUSE
        | MOD_FEAR
        | TRANSFORM
        | MOD_SHAPESHIFT
        | MOUNTED
        | FLY
        | MOD_INCREASE_MOUNTED_FLIGHT_SPEED => UnitState,
        ADD_FLAT_MODIFIER
        | ADD_PCT_MODIFIER
        | ADD_PCT_MODIFIER_BY_SPELL_LABEL
        | ADD_FLAT_MODIFIER_BY_SPELL_LABEL
        | MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN
        | MOD_COOLDOWN_BY_HASTE_REGEN
        | MOD_SPELL_CATEGORY_COOLDOWN
        | MOD_RECOVERY_RATE
        | MOD_RECOVERY_RATE_BY_SPELL_LABEL => SpellMod,
        PROC_TRIGGER_SPELL | PROC_TRIGGER_SPELL_WITH_VALUE => ProcTrigger,
        MOD_POWER_COST_SCHOOL_PCT | MOD_ADDITIONAL_POWER_COST => PowerCost,
        ALLOW_TALENT_SWAPPING => TalentSwap,
        SCHOOL_ABSORB => Absorb,
        SCHOOL_IMMUNITY => SchoolImmunity,
        MECHANIC_IMMUNITY | MECHANIC_IMMUNITY_MASK => MechanicImmunity,
        EFFECT_IMMUNITY => EffectImmunity,
        MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER
        | MOD_SPELL_DAMAGE_FROM_CASTER
        | MOD_SPELL_DAMAGE_FROM_CASTER_BY_LABEL => DamageFromCaster,
        MOD_MAX_CHARGES
        | MOD_CHARGE_RECOVERY_RATE
        | CHARGE_RECOVERY_MOD
        | CHARGE_RECOVERY_MULTIPLIER
        | CHARGE_RECOVERY_AFFECTED_BY_HASTE_REGEN => ChargeMod,
        REFLECT_SPELLS | REFLECT_SPELLS_SCHOOL => Reflect,
        MOD_ATTACKER_MELEE_HIT_CHANCE
        | MOD_ATTACKER_RANGED_HIT_CHANCE
        | MOD_ATTACKER_SPELL_HIT_CHANCE
        | MOD_ATTACKER_MELEE_CRIT_CHANCE
        | DEFLECT_SPELLS
        | IGNORE_HIT_DIRECTION => HitDefense,
        _ => return None,
    })
}

pub fn is_handled(aura_type: u32) -> bool {
    kind(aura_type).is_some()
}

/// TrinityCore `isTriggerAura` types this runtime handles: an aura procs
/// only when one of its effects is one of these.
pub fn can_proc(aura_type: u32) -> bool {
    matches!(
        aura_type,
        DUMMY
            | PERIODIC_DUMMY
            | MOD_CONFUSE
            | MOD_FEAR
            | TRANSFORM
            | PROC_TRIGGER_SPELL
            | PROC_TRIGGER_SPELL_WITH_VALUE
            | MOD_STUN
            | MOD_ROOT
            | MOD_ROOT_2
            | SCHOOL_ABSORB
            | MOD_DAMAGE_PERCENT_TAKEN
            | MOD_DAMAGE_PERCENT_DONE
            | MOD_SPELL_DAMAGE_FROM_CASTER
            | MOD_POWER_REGEN_PERCENT
            | MOD_MELEE_HASTE
            | REFLECT_SPELLS
            | REFLECT_SPELLS_SCHOOL
    )
}

/// SPELL_AURA_REFLECT_SPELLS(_SCHOOL): auras that proc only off a reflect
/// (`SpellMgr::LoadSpellProcs` `PROC_HIT_REFLECT`, SpellMgr.cpp:1847-1851).
pub fn reflects(aura_type: u32) -> bool {
    matches!(aura_type, REFLECT_SPELLS | REFLECT_SPELLS_SCHOOL)
}

/// Crowd control that damage breaks (TrinityCore `AuraEffect::HandleBreakableCCAuraProc`):
/// with `ProcTypeMask` set, the amount is 10% of the target's max health
/// (`AuraEffect::CalculateAmount`) and each damaging proc takes the damage off it
/// (`CheckEffectProc` requires damage). MOD_ROOT_2 is in `HandleProc`'s list but not
/// `isTriggerAura`, so it never procs there.
pub fn breaks_on_damage(aura_type: u32) -> bool {
    matches!(
        aura_type,
        MOD_CONFUSE | MOD_FEAR | MOD_STUN | MOD_ROOT | TRANSFORM
    )
}

/// Types whose amount adds SP/AP coefficients (TrinityCore applies spell
/// power only to damage, heal and absorb amounts); others use base points.
pub fn scales_with_power(aura_type: u32) -> bool {
    matches!(
        aura_type,
        PERIODIC_DAMAGE | PERIODIC_LEECH | PERIODIC_HEAL | SCHOOL_ABSORB
    )
}
