//! Temporary actors cast only locally bound creature spells. No guessed rotations.
use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::casting::CastState;
use shared::components::{Npc, PowerType};

use super::summon::SummonedCreature;

#[path = "guardian_core.rs"]
pub(super) mod core;
#[path = "guardian_implosion.rs"]
pub(super) mod implosion;
#[path = "guardian_tyrant.rs"]
pub(super) mod tyrant;

const SUMMON_DEMONIC_TYRANT: u32 = 265187;
const ANTORAN_ARMAMENTS: u32 = 1250921;

// SimC76f8118 pets.cpp701/875–882: the actual Wild Imp consumes its
// initial native Energy without regeneration, then fades when exhausted.
const NO_POWER_REGEN: &[(u32, u32)] = &[(104317, 55659), (279910, 143622)];

struct GuardianPolicy {
    summons: &'static [u32],
    entry: u32,
    spells: &'static [u32],
    repeat_gcd: Option<f64>,
    owner_spell_share: Option<f32>,
    owner_attack_share: Option<f32>,
}

// SimC76f8118 pets.cpp19/705/867/1510–1548: bindings and inheritance
// belong to the actual root/creature pair, including the alternative Tyrant.
const GUARDIAN_POLICIES: &[GuardianPolicy] = &[
    // SimC76f8118 sc_monk_pets.cpp425–435: Niuzao inherits its owner's AP.
    GuardianPolicy {
        summons: &[132578],
        entry: 73967,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(1.0),
    },
    // SimC76f8118 sc_death_knight.cpp4709–4780: Blood Beast casts
    // Corrupted Blood and inherits three times the owner's melee AP.
    GuardianPolicy {
        summons: &[434237],
        entry: 217228,
        spells: &[434574],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(3.0),
    },
    // SimC76f8118 sc_death_knight.cpp4295–4304: each Dancing Rune Weapon
    // inherits one third of its owner's live melee attack power.
    GuardianPolicy {
        summons: &[49028, 1237128, 1264353],
        entry: 27893,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(1.0 / 3.0),
    },
    // SimC76f8118 sc_death_knight.cpp4478–4487/4599–4605: actual Magus AP share
    // and MID2 actions; each action retains its native cast and recovery data.
    GuardianPolicy {
        summons: &[317776, 1242294],
        entry: 163366,
        spells: &[1297055, 1297091],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(0.4664),
    },
    // SimC76f8118 sc_death_knight.cpp4632–4680: the real Lord uses
    // Necrotic Bolt with 0.7 of its owner's attack power in MID2.
    GuardianPolicy {
        summons: &[1292072],
        entry: 264321,
        spells: &[1297086],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(0.7),
    },
    // SimC76f8118 sc_death_knight.cpp5478–5488: the actual Abomination
    // inherits its owner's melee attack power.
    GuardianPolicy {
        summons: &[288853],
        entry: 149555,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(2.34),
    },
    // SimC76f8118 sc_death_knight.cpp3621–3649: ordinary and Army
    // lesser ghouls inherit owner melee AP; Army gets the 1.4 multiplier.
    GuardianPolicy {
        summons: &[275430],
        entry: 237409,
        spells: &[1278150, 199373],
        repeat_gcd: Some(1.5),
        owner_spell_share: None,
        owner_attack_share: Some(0.180431625),
    },
    GuardianPolicy {
        summons: &[1294026],
        entry: 237409,
        spells: &[1278150, 199373],
        repeat_gcd: Some(1.5),
        owner_spell_share: None,
        owner_attack_share: Some(0.180431625 * 1.4),
    },
    // SimC76f8118 sc_death_knight.cpp4849–4855: each actual Horseman
    // inherits 0.70125 of its owner's melee attack power.
    GuardianPolicy {
        summons: &[444248],
        entry: 221632,
        spells: &[445504],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(0.70125),
    },
    GuardianPolicy {
        summons: &[444251],
        entry: 221633,
        spells: &[445513],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(0.70125),
    },
    GuardianPolicy {
        summons: &[444252],
        entry: 221634,
        spells: &[445508],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(0.70125),
    },
    GuardianPolicy {
        summons: &[444254],
        entry: 221635,
        spells: &[444826, 445507],
        repeat_gcd: Some(1.0),
        owner_spell_share: None,
        owner_attack_share: Some(0.70125),
    },
    // SimC76f8118 sc_hunter.cpp1935: Dire Beast inherits live owner AP1.0.
    GuardianPolicy {
        summons: &[219199],
        entry: 110063,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(1.0),
    },
    // SimC76f8118 sc_evoker.cpp3795–3797: native Commandos inherit live owner SP.
    GuardianPolicy {
        summons: &[1235843, 1237036],
        entry: 219827,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 pets.cpp2050–2119: actual Infernal, native self-buff
    // clocks/children, and validated SP/AP from live owner spell power.
    GuardianPolicy {
        summons: &[111685],
        entry: 89,
        spells: &[264364, 19483],
        repeat_gcd: None,
        owner_spell_share: Some(2.2275),
        owner_attack_share: Some(2.2275),
    },
    // SimC76f8118 sc_hunter.cpp1973/2008: Fenryr and Hati inherit live owner AP2.0.
    GuardianPolicy {
        summons: &[459733],
        entry: 228224,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(2.0),
    },
    GuardianPolicy {
        summons: &[459739],
        entry: 228226,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(2.0),
    },
    // SimC76f8118 sc_hunter.cpp2043–2047: Bear inherits live owner AP0.6.
    // Rend is explicitly launched on summon, not an autonomous guardian cast.
    GuardianPolicy {
        summons: &[471990],
        entry: 234018,
        spells: &[],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(0.6),
    },
    GuardianPolicy {
        summons: &[104317],
        entry: 55659,
        spells: &[104318],
        repeat_gcd: None,
        owner_spell_share: Some(1.2),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[279910],
        entry: 143622,
        spells: &[104318],
        repeat_gcd: None,
        owner_spell_share: Some(1.2),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[SUMMON_DEMONIC_TYRANT],
        entry: 135002,
        spells: &[270481],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // pets.cpp1541–1543: Burning Cleave is a hasted two-second GCD attack,
    // not an untimed once-only cast or an added ordinary white swing.
    GuardianPolicy {
        summons: &[SUMMON_DEMONIC_TYRANT],
        entry: 250289,
        spells: &[1264093],
        repeat_gcd: Some(2.0),
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 pets.cpp1033–1050/1081–1108/1227: one initial Dreadbite;
    // AP inheritance is validated there; do not copy unvalidated SP1.3.
    GuardianPolicy {
        summons: &[193331, 193332],
        entry: 98035,
        spells: &[271971],
        repeat_gcd: None,
        owner_spell_share: None,
        owner_attack_share: Some(1.06769),
    },
    // pets.cpp1672–1704: native Greater Felbolt and validated SP2.75.
    GuardianPolicy {
        summons: &[1276452],
        entry: 258584,
        spells: &[1277116],
        repeat_gcd: None,
        owner_spell_share: Some(2.75),
        owner_attack_share: None,
    },
    // pets.cpp1836–2040: Dominion actors inherit validated owner SP1.0;
    // each native action supplies its cast/channel clock and damage children.
    GuardianPolicy {
        summons: &[1282501],
        entry: 260076,
        spells: &[1282507],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[1282502],
        entry: 260077,
        spells: &[1282533],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[1276283],
        entry: 258242,
        spells: &[1280457],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[1276182],
        entry: 255147,
        spells: &[1292384],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 pets.cpp17–19/2577–2591: Diabolist demons inherit base owner
    // SP1.0; the Overlord's validated AP share is1.0. Native rows own the actions.
    GuardianPolicy {
        summons: &[428571],
        entry: 228575,
        spells: &[432120],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: Some(1.0),
    },
    GuardianPolicy {
        summons: &[428565],
        entry: 228576,
        spells: &[432569],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[434400],
        entry: 228574,
        spells: &[438973],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 pets.cpp1310–1342: validated variant inheritance. Native
    // Bile Spit/Headbutt remain distinct from the melee-linked Gloom Slash.
    GuardianPolicy {
        summons: &[1251781],
        entry: 135816,
        spells: &[267997, 267999],
        repeat_gcd: None,
        owner_spell_share: Some(1.94),
        owner_attack_share: Some(0.45),
    },
    GuardianPolicy {
        summons: &[455465],
        entry: 226268,
        spells: &[267997, 267999],
        repeat_gcd: None,
        owner_spell_share: Some(2.619),
        owner_attack_share: Some(0.6075),
    },
    GuardianPolicy {
        summons: &[455476],
        entry: 226269,
        spells: &[267997, 267999],
        repeat_gcd: None,
        owner_spell_share: Some(1.94),
        owner_attack_share: Some(0.45),
    },
    // Native205180 description names $205231s1; preserve the existing binding.
    // SimC76f8118 pets.cpp19/2424–2430: Darkglare retains base owner SP1.0.
    GuardianPolicy {
        summons: &[205180],
        entry: 103673,
        spells: &[205231],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 pets.cpp19/2358–2407: actual Overfiend repeatedly
    // casts its native Chaos Bolt with base owner SP1.0; no guessed AP share.
    GuardianPolicy {
        summons: &[434587],
        entry: 217429,
        spells: &[434589],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 pets19/2516–2563: native Desperate Soul, full SP,
    // one actual Wrath; its own post-cast callback marks native dismissal.
    GuardianPolicy {
        summons: &[1262094],
        entry: 254505,
        spells: &[1278047],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 sc_priest_pets.cpp75–86/1061–1134: native Thing From
    // Beyond repeats Void Spike with full owner SP; native cast time owns cadence.
    GuardianPolicy {
        summons: &[373277],
        entry: 189988,
        spells: &[373279],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 sc_shaman.cpp4064–4139/4248–4294: greater variants
    // inherit owner SP1.0 and omit the Primal-only Meteor/Immolate/Stormfury.
    GuardianPolicy {
        summons: &[188592],
        entry: 95061,
        spells: &[57984],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[157299],
        entry: 77936,
        spells: &[157348, 157331],
        repeat_gcd: None,
        owner_spell_share: Some(1.0),
        owner_attack_share: None,
    },
    // SimC76f8118 sc_shaman.cpp4026–4042/4070–4139/4248–4315: Primal Fire and
    // Storm APL order with owner SP1.0 x (1 + native Primal Elementalist117013
    // effect0 80%); these native roots exist only with that talent.
    GuardianPolicy {
        summons: &[118291],
        entry: 61029,
        spells: &[117588, 118297, 57984],
        repeat_gcd: None,
        owner_spell_share: Some(1.8),
        owner_attack_share: None,
    },
    GuardianPolicy {
        summons: &[157319],
        entry: 77942,
        spells: &[157375, 157348, 157331],
        repeat_gcd: None,
        owner_spell_share: Some(1.8),
        owner_attack_share: None,
    },
];

fn policy(summon: u32, entry: u32) -> Option<&'static GuardianPolicy> {
    GUARDIAN_POLICIES
        .iter()
        .find(|row| row.summons.contains(&summon) && row.entry == entry)
}

pub(crate) fn owner_spell_share(summon: u32, entry: u32) -> Option<f32> {
    policy(summon, entry).and_then(|row| row.owner_spell_share)
}

pub(crate) fn owner_attack_share(summon: u32, entry: u32) -> Option<f32> {
    policy(summon, entry).and_then(|row| row.owner_attack_share)
}

// SimC76f8118 pets.cpp372–375/421–430/663–668: controlled demons
// inherit their spell and attack power from live owner SP.
const CONTROLLED_PET_POWER_SHARES: &[(u32, f32, f32)] = &[
    (416, 5.625, 2.8125),
    (1863, 5.175, 2.565),
    (17252, 1.74228, 1.82412),
];

pub(crate) fn controlled_pet_power_shares(entry: u32) -> Option<(f32, f32)> {
    CONTROLLED_PET_POWER_SHARES
        .iter()
        .find(|(creature, _, _)| *creature == entry)
        .map(|(_, spell, attack)| (*spell, *attack))
}

// SimC76f8118 pets.cpp1513/1520–1523 selects the Antoran weapon mode.
// Native1264093 requires1251000; grant that real mode instead of bypassing
// its caster-aura gate. pets.cpp56–57/1714–1719 grants Service to Imp Lord.
const SPAWN_AURAS: &[(u32, u32, &[u32])] = &[
    (SUMMON_DEMONIC_TYRANT, 250289, &[1251000]),
    (1276452, 258584, &[216187]),
];

fn apply_spawn_auras(world: &mut World, actor: Entity, root: u32, entry: u32) {
    for (_, _, auras) in SPAWN_AURAS
        .iter()
        .filter(|(spell, creature, _)| *spell == root && *creature == entry)
    {
        for &aura in *auras {
            if !super::try_cast_triggered(world, actor, aura, Some(actor), 0) {
                warn!(?actor, aura, "Native guardian spawn aura refused");
            }
        }
    }
}

struct GuardianActionPolicy {
    entries: &'static [u32],
    spell: u32,
    once: bool,
    cooldown: Option<f64>,
}
// SimC76f8118 pets.cpp1350/1382–1418: one Bile Spit per summon and an
// unhasted five-second Headbutt cooldown. Native spell rows own the payloads.
const GUARDIAN_ACTIONS: &[GuardianActionPolicy] = &[
    // SimC76f8118 pets.cpp2626–2644/2758–2786/2862–2892: each Diabolist demon
    // executes its single Wicked Cleave, Chaos Salvo or Felseeker once.
    GuardianActionPolicy {
        entries: &[228575],
        spell: 432120,
        once: true,
        cooldown: None,
    },
    GuardianActionPolicy {
        entries: &[228576],
        spell: 432569,
        once: true,
        cooldown: None,
    },
    GuardianActionPolicy {
        entries: &[228574],
        spell: 438973,
        once: true,
        cooldown: None,
    },
    GuardianActionPolicy {
        entries: &[135816, 226268, 226269],
        spell: 267997,
        once: true,
        cooldown: None,
    },
    GuardianActionPolicy {
        entries: &[135816, 226268, 226269],
        spell: 267999,
        once: false,
        cooldown: Some(5.0),
    },
];

fn action_policy(entry: u32, spell: u32) -> Option<&'static GuardianActionPolicy> {
    GUARDIAN_ACTIONS
        .iter()
        .find(|row| row.spell == spell && row.entries.contains(&entry))
}

struct GuardianPowerPolicy {
    entry: u32,
    power: PowerType,
    maximum: f32,
    cast_threshold: f32,
}
// SimC76f8118 pets.cpp1680–1705: a new Imp Lord starts with200 Energy
// and waits for125 before Greater Felbolt; native SpellPower owns its40 cost
// and native PowerType owns regeneration. This is not an owner-pool reset.
const GUARDIAN_POWERS: &[GuardianPowerPolicy] = &[GuardianPowerPolicy {
    entry: 258584,
    power: PowerType::Energy,
    maximum: 200.0,
    cast_threshold: 125.0,
}];

/// Source-defined guardian bases also govern later native maximum-power folds.
pub(super) fn power_base(entry: u32, power: PowerType) -> Option<f32> {
    GUARDIAN_POWERS
        .iter()
        .find(|policy| policy.entry == entry && policy.power == power)
        .map(|policy| policy.maximum)
}

/// SimC76f8118 pets1513: Antoran Armaments selects the alternative native root effect.
pub(super) fn effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    if spell != SUMMON_DEMONIC_TYRANT || !matches!(index, 0 | 3) {
        return true;
    }
    let antoran = world
        .get::<crate::class_progression::KnownSpells>(caster)
        .is_some_and(|known| known.contains(ANTORAN_ARMAMENTS));
    (index == 3) == antoran
}

#[derive(Resource, Default)]
pub struct GuardianSpells {
    pub spells: HashMap<u32, Vec<u32>>,
    spirit_wolf_attack_time: Option<f32>,
}

/// TC TemporarySummon.h:119 / Pet.cpp:1026: Spirit Wolf29264 only.
/// Intrinsic AP needs absent pet_levelstats; never substitute fake Strength22.
#[derive(Component)]
pub(crate) struct SpiritWolf {
    pub owner: Entity,
    pub weapon: crate::item_stats::WeaponDamage,
}

impl SpiritWolf {
    /// StatSystem.cpp:1353: owner contribution, with native integer AP/SP storage.
    pub fn owner_power(attack_power: f32) -> f32 {
        (attack_power * 0.31) as i32 as f32
    }
}

pub fn load_guardian_spells(conn: &Connection) -> rusqlite::Result<GuardianSpells> {
    let mut statement = conn
        .prepare("SELECT CreatureID, Spell FROM guardian_spells ORDER BY CreatureID, SpellIndex")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))?;
    let mut result = GuardianSpells::default();
    for row in rows {
        let (entry, spell) = row?;
        let spells = result.spells.entry(entry).or_default();
        if !spells.contains(&spell) {
            spells.push(spell);
        }
    }
    if result.spells.contains_key(&29264) {
        let attack_time: u32 = conn.query_row(
            "SELECT BaseAttackTime FROM creature_template WHERE entry = 29264",
            [],
            |row| row.get(0),
        )?;
        result.spirit_wolf_attack_time = Some(attack_time as f32 / 1000.0);
    }
    Ok(result)
}

pub(super) fn load_startup(mut commands: Commands) {
    let path = std::path::Path::new(crate::spell_info::WORLD_DB_PATH);
    if !path.exists() {
        warn!("world.db absent: guardian spell bindings unavailable");
        return;
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open guardian spell bindings");
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='guardian_spells' AND type='table')",
        [], |row| row.get(0),
    ).expect("check guardian spell table");
    if !exists {
        warn!(
            "guardian_spells not imported; run scripts/import-guardian-spells.py against local TDB archive"
        );
        return;
    }
    commands.insert_resource(load_guardian_spells(&conn).expect("load guardian spells"));
}

#[derive(Component)]
pub(crate) struct GuardianActor {
    pub(crate) spells: Vec<u32>,
    pub(super) regenerates_power: bool,
    /// Untimed actions already attempted; source-once casts use real history.
    once_used: Vec<u32>,
    spell_ready_at: HashMap<u32, f64>,
    repeat_gcd: Option<f64>,
    ready_at: f64,
}

/// Actual executing guardians read their owner's live secondary stats.
/// SimC76f8118 pet.cpp217–226; no creature template or class identity changes.
pub(crate) fn stat_owner(world: &World, actor: Entity) -> Option<Entity> {
    let summon = world.get::<SummonedCreature>(actor)?;
    // SimC76f8118 sc_death_knight.cpp5478–5504: Abomination is a pet
    // despite having no autonomous guardian spell binding.
    let abomination = summon.spell_id == 288853
        && world
            .get::<Npc>(actor)
            .is_some_and(|npc| npc.template_id == 149555);
    (abomination
        || rune_weapon_owner(world, actor).is_some()
        || world.get::<GuardianActor>(actor).is_some())
    .then_some(summon.owner)
}

/// SimC76f8118 sc_death_knight.cpp4295–4304: actual rune weapons are pets
/// even though they do not have autonomous guardian spell bindings.
pub(crate) fn rune_weapon_owner(world: &World, actor: Entity) -> Option<Entity> {
    let summon = world.get::<SummonedCreature>(actor)?;
    (matches!(summon.spell_id, 49028 | 1237128 | 1264353)
        && world
            .get::<Npc>(actor)
            .is_some_and(|npc| npc.template_id == 27893))
    .then_some(summon.owner)
}

/// SimC76f8118 pets.cpp2584: the Overlord is not affected by the talent.
pub(crate) fn is_diabolist_overlord(world: &World, actor: Entity) -> bool {
    world
        .get::<SummonedCreature>(actor)
        .is_some_and(|summon| summon.spell_id == 428571)
}

/// SimC76f8118 pets.cpp185–201: bugs=1 applies the native talent's
/// effect2 twice to each actual warlock demon's critical damage bonus.
pub(super) fn demonic_brutality_critical_bonus(world: &World, actor: Entity) -> f32 {
    const DEMONIC_BRUTALITY: u32 = 453908;
    if world
        .get::<crate::hunter_pet::SummonedDemon>(actor)
        .is_none()
        || is_diabolist_overlord(world, actor)
    {
        return 0.0;
    }
    // SimC76f8118 pets.cpp1845 excludes Dominion of Argus guardians.
    if world
        .get::<SummonedCreature>(actor)
        .is_some_and(|summon| matches!(summon.spell_id, 1282501 | 1282502 | 1276283 | 1276182))
    {
        return 0.0;
    }
    let Some((owner, _)) = crate::auras::summon_damage::ownership(world, actor) else {
        return 0.0;
    };
    if !world
        .get::<crate::class_progression::KnownSpells>(owner)
        .is_some_and(|known| known.contains(DEMONIC_BRUTALITY))
    {
        return 0.0;
    }
    2.0 * super::scripts::script_effect_value(world, owner, DEMONIC_BRUTALITY, 1) / 100.0
}

pub(super) fn initialize(world: &mut World, actor: Entity) {
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    if summon.properties.is_totem() {
        initialize_passive_totem(world, actor);
        return;
    }
    // Mainline49206 uses pet-title3238, but is a temporary Gargoyle actor.
    if !summon.properties.is_guardian() && summon.spell_id != 49206 {
        return;
    }
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    let owner = summon.owner;
    if entry == 29264 {
        initialize_spirit_wolf(world, actor, owner);
    }
    tyrant::on_spawn(world, actor);
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    let root = summon.spell_id;
    let policy = policy(root, entry).filter(|_| {
        !matches!((root, entry), (1242294, 163366) | (1292072, 264321))
            || crate::auras::has_aura(world, owner, 1296654)
    });
    let spells = policy.map_or_else(
        || {
            world
                .resource::<GuardianSpells>()
                .spells
                .get(&entry)
                .cloned()
                .unwrap_or_default()
        },
        |row| row.spells.to_vec(),
    );
    if spells.is_empty() {
        warn!(
            "Guardian creature {entry}: no local creature spell binding; actor has no guessed attack"
        );
        return;
    }
    let pools = load_actor_powers(world, entry, &spells);
    let demon = world
        .resource::<crate::world_data::WorldData>()
        .template_for(entry, 0)
        .is_some_and(|template| template.creature_type == 3);
    world.entity_mut(actor).insert((
        GuardianActor {
            spells,
            regenerates_power: !NO_POWER_REGEN.contains(&(root, entry)),
            once_used: Vec::new(),
            spell_ready_at: HashMap::new(),
            repeat_gcd: policy.and_then(|row| row.repeat_gcd),
            ready_at: 0.0,
        },
        pools,
    ));
    if demon {
        world
            .entity_mut(actor)
            .insert(crate::hunter_pet::SummonedDemon);
    }
    apply_spawn_auras(world, actor, root, entry);
    implosion::on_spawn(world, actor);
}

fn initialize_passive_totem(world: &mut World, actor: Entity) {
    // TC a352b1fa Totem.cpp InitStats/InitSummon: an instant first
    // creature spell is passive; the second creature spell is always cast.
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    let spells = world
        .resource::<GuardianSpells>()
        .spells
        .get(&entry)
        .cloned()
        .unwrap_or_default();
    for (index, id) in spells.into_iter().take(2).enumerate() {
        let Some(spell) = world.resource::<crate::spell_info::SpellStore>().get(id) else {
            warn!(entry, id, "Totem creature spell is absent");
            continue;
        };
        if index == 0 && super::cast_time_ms(world, actor, spell) != 0 {
            continue;
        }
        super::cast_triggered(world, actor, id, Some(actor), 1);
    }
}

fn initialize_spirit_wolf(world: &mut World, actor: Entity, owner: Entity) {
    let speed = world
        .resource::<GuardianSpells>()
        .spirit_wolf_attack_time
        .expect("Spirit Wolf requires sourced creature_template.BaseAttackTime");
    assert!(speed > 0.0, "Spirit Wolf BaseAttackTime must be positive");
    let level = f32::from(world.get::<shared::components::UnitLevel>(actor).unwrap().0);
    world.entity_mut(actor).insert(SpiritWolf {
        owner,
        weapon: crate::item_stats::WeaponDamage {
            min: 3.0 * level,
            max: 5.0 * level,
            speed,
            kind: shared::components::WeaponType::OneHand,
        },
    });
}

fn load_actor_powers(world: &World, entry: u32, spells: &[u32]) -> super::powers::PowerPools {
    let store = world.resource::<crate::spell_info::SpellStore>();
    let mut powers = Vec::new();
    for id in spells {
        let Some(spell) = store.get(*id) else {
            warn!("Guardian creature {entry}: bound spell {id} absent");
            continue;
        };
        for power in spell
            .power_costs
            .iter()
            .filter_map(|cost| PowerType::from_db(cost.power_type))
        {
            if !powers.contains(&power) {
                powers.push(power);
            }
        }
    }
    let mut pools = super::powers::PowerPools::from_table(
        world.resource::<super::powers::PowerTypeTable>(),
        powers,
    );
    for policy in GUARDIAN_POWERS
        .iter()
        .filter(|policy| policy.entry == entry)
    {
        let pool = pools
            .get_mut(policy.power)
            .expect("source-bound guardian requires its native action's pool");
        pool.max = policy.maximum;
        pool.current = policy.maximum;
    }
    pools
}

/// Initialize existing action clocks once; sampling never runs for old actors.
/// Source leaves supply delays, while native casts and deadlines stay authoritative.
pub fn initialize_action_delays(
    world: &mut World,
    actor: Entity,
    sample: impl FnOnce() -> Vec<(u32, f64)>,
) {
    let now = super::now(world);
    let mut state = world
        .get_mut::<GuardianActor>(actor)
        .expect("source-bound action delays require an initialized native guardian");
    let initialized = !state.spell_ready_at.is_empty() || !state.once_used.is_empty();
    if initialized {
        return;
    }
    for (spell, delay) in sample() {
        assert!(
            state.spells.contains(&spell),
            "initial clock requires a bound native action"
        );
        state.spell_ready_at.insert(spell, now + delay);
    }
}

/// Reset every bound action's cooldown, e.g. when a source re-summons an active
/// guardian (SimC76f8118 sc_shaman.cpp11561–11564); cast state is unchanged.
pub fn reset_action_cooldowns(world: &mut World, actor: Entity) {
    let mut state = world
        .get_mut::<GuardianActor>(actor)
        .expect("cooldown reset requires an initialized native guardian");
    state.spell_ready_at.clear();
    let spells = state.spells.clone();
    if let Some(mut history) = world.get_mut::<super::SpellHistory>(actor) {
        for id in spells {
            history.spell_ready_at.remove(&id);
        }
    }
}

pub(super) fn update(world: &mut World) {
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<GuardianActor>>()
        .iter(world)
        .collect();
    for actor in actors {
        if implosion::advance(world, actor) {
            continue;
        }
        if wild_imp_exhausted(world, actor) {
            if !implosion::start_isolated(world, actor) {
                let owner = world.get::<SummonedCreature>(actor).unwrap().owner;
                core::on_natural_fade(world, owner);
                super::summon::despawn_summon(world, actor);
            }
        } else {
            cast_on_owner_target(world, actor);
        }
    }
}

fn wild_imp_exhausted(world: &World, actor: Entity) -> bool {
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    if !matches!((summon.spell_id, entry), (104317, 55659) | (279910, 143622))
        || world.get::<CastState>(actor).is_some()
        || world
            .resource::<super::missiles::PendingSpellHits>()
            .has_caster(actor)
    {
        return false;
    }
    cannot_afford_firebolt(world, actor)
}

pub(super) fn cannot_afford_firebolt(world: &World, actor: Entity) -> bool {
    let bolt = world
        .resource::<crate::spell_info::SpellStore>()
        .get(104318)
        .unwrap();
    let costs = super::applicable_costs(world, actor, bolt)
        .expect("native Wild Imp requires its Energy pool and Fel Firebolt cost");
    // SimC76f8118 pets.cpp833–849: dismiss when no next bolt is affordable.
    // Existing missiles require a live source; drain them before retirement.
    costs.iter().any(|(pool, cost, current)| {
        *pool == super::CostPool::Power(PowerType::Energy) && current < cost
    })
}

fn cast_on_owner_target(world: &mut World, actor: Entity) {
    let owner = world.get::<SummonedCreature>(actor).unwrap().owner;
    let target = world
        .get::<crate::networking::CombatTarget>(owner)
        .and_then(|target| target.0);
    world
        .get_mut::<crate::networking::CombatTarget>(actor)
        .unwrap()
        .0 = target;
    if world.get::<CastState>(actor).is_some() || !super::targets::is_alive(world, actor) {
        return;
    }
    let Some(target) = target.filter(|target| {
        super::targets::is_alive(world, *target)
            && world.get::<crate::maps::InMap>(*target) == world.get::<crate::maps::InMap>(actor)
    }) else {
        return;
    };
    cast_first_ready(world, actor, target);
}

fn has_repeat_clock(spell: &crate::spell_info::SpellInfo) -> bool {
    let has_cast_time = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.cast_time)
        .is_some_and(|time| time.base_ms > 0);
    let has_cooldown = spell
        .cooldowns
        .as_ref()
        .is_some_and(|cooldowns| cooldowns.recovery_ms > 0 || cooldowns.category_recovery_ms > 0);
    has_cast_time || has_cooldown
}

fn cast_first_ready(world: &mut World, actor: Entity, target: Entity) {
    let now = super::now(world);
    if world.get::<GuardianActor>(actor).unwrap().ready_at > now {
        return;
    }
    if !power_threshold_ready(world, actor) {
        return;
    }
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    let spells = world.get::<GuardianActor>(actor).unwrap().spells.clone();
    for id in spells {
        let Some(spell) = world.resource::<crate::spell_info::SpellStore>().get(id) else {
            continue;
        };
        let repeatable = can_repeat(entry, spell);
        let targets = native_cast_targets(actor, target, spell);
        if !read_action_timing_ready(world, actor, id, repeatable)
            || !super::script_registry::guardian_action_eligible(world, actor, id, target)
        {
            continue;
        }
        if super::creature_cast_with_targets(world, actor, id, targets).is_err() {
            continue;
        }
        start_repeat_clock(world, actor, id, repeatable);
        break;
    }
}

fn native_cast_targets(
    actor: Entity,
    enemy: Entity,
    spell: &crate::spell_info::SpellInfo,
) -> super::CastTargets {
    // SimC76f8118 pets.cpp1534–1548 supplies Burning Cleave's AoE policy;
    // native18/16 selectors center it on the caster. Such spells have no
    // explicit unit: do not range-check the enemy as the cast unit. Preserve
    // that enemy separately as the actual selected primary.
    // Dreadbite's source-qualified target plan requires its selected enemy.
    let unit = if spell.id == 271971
        || super::targets::explicit_requirement(spell.effects.iter()).is_some()
    {
        enemy
    } else {
        actor
    };
    super::CastTargets {
        unit,
        primary: Some(enemy),
        destination: None,
    }
}

fn can_repeat(entry: u32, spell: &crate::spell_info::SpellInfo) -> bool {
    let policy = action_policy(entry, spell.id);
    !policy.is_some_and(|row| row.once)
        && (has_repeat_clock(spell) || policy.is_some_and(|row| row.cooldown.is_some()))
}

fn read_action_timing_ready(world: &World, actor: Entity, id: u32, repeatable: bool) -> bool {
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    if action_policy(entry, id).is_some_and(|row| row.once) {
        // A cast start is not success: interrupted Bile Spit remains available.
        return !world
            .get::<super::SpellHistory>(actor)
            .is_some_and(|history| history.last_cast_in(&[id]).is_some());
    }
    let state = world.get::<GuardianActor>(actor).unwrap();
    let repeat_allowed = repeatable || state.repeat_gcd.is_some() || !state.once_used.contains(&id);
    let cooldown_ready = state
        .spell_ready_at
        .get(&id)
        .is_none_or(|until| *until <= super::now(world));
    repeat_allowed && cooldown_ready
}

fn power_threshold_ready(world: &World, actor: Entity) -> bool {
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    GUARDIAN_POWERS
        .iter()
        .filter(|policy| policy.entry == entry)
        .all(|policy| {
            super::powers::power_value(world, actor, policy.power)
                .expect("source-bound guardian requires its authoritative native pool")
                .0
                >= policy.cast_threshold
        })
}

fn start_repeat_clock(world: &mut World, actor: Entity, id: u32, repeatable: bool) {
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    if let Some(cooldown) = action_policy(entry, id).and_then(|row| row.cooldown) {
        let until = super::now(world) + cooldown;
        world
            .get_mut::<GuardianActor>(actor)
            .unwrap()
            .spell_ready_at
            .insert(id, until);
    }
    if let Some(gcd) = world.get::<GuardianActor>(actor).unwrap().repeat_gcd {
        let owner = world.get::<SummonedCreature>(actor).unwrap().owner;
        let haste = world
            .get::<crate::stats::CharacterStats>(owner)
            .expect("source-bound guardian requires its owner's resolved stats")
            .haste_pct;
        let ready_at = super::now(world) + gcd / (1.0 + f64::from(haste) / 100.0);
        world.get_mut::<GuardianActor>(actor).unwrap().ready_at = ready_at;
    } else if !repeatable && !action_policy(entry, id).is_some_and(|row| row.once) {
        world
            .get_mut::<GuardianActor>(actor)
            .unwrap()
            .once_used
            .push(id);
    }
}
