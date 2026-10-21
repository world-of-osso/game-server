//! Temporary actors cast only locally bound creature spells. No guessed rotations.
use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::casting::CastState;
use shared::components::{Npc, PowerType};

use super::summon::SummonedCreature;

#[path = "guardian_core.rs"]
pub(super) mod core;

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
pub(super) struct GuardianActor {
    spells: Vec<u32>,
    /// Untimed spells can be attempted once, never assigned a synthetic cadence.
    untimed_used: Vec<u32>,
}

pub(super) fn initialize(world: &mut World, actor: Entity) {
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    // Mainline49206 uses pet-title3238, but is a temporary Gargoyle actor.
    if !summon.properties.is_guardian() && summon.spell_id != 49206 {
        return;
    }
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    let owner = summon.owner;
    if entry == 29264 {
        initialize_spirit_wolf(world, actor, owner);
    }
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    let spells = match (summon.spell_id, entry) {
        // SimC76f8118 sc_warlock_pets.cpp721/867: sourced Wild Imp only.
        (104317, 55659) => vec![104318],
        // Native205180 description explicitly names $205231s1. TDB1210 has
        // no103673 creature spell row; this is not an Imp/Firebolt substitute.
        (205180, 103673) => vec![205231],
        _ => world
            .resource::<GuardianSpells>()
            .spells
            .get(&entry)
            .cloned()
            .unwrap_or_default(),
    };
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
            untimed_used: Vec::new(),
        },
        pools,
    ));
    if demon {
        world
            .entity_mut(actor)
            .insert(crate::hunter_pet::SummonedDemon);
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
    super::powers::PowerPools::from_table(world.resource::<super::powers::PowerTypeTable>(), powers)
}

pub(super) fn update(world: &mut World) {
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<GuardianActor>>()
        .iter(world)
        .collect();
    for actor in actors {
        if wild_imp_exhausted(world, actor) {
            let owner = world.get::<SummonedCreature>(actor).unwrap().owner;
            core::on_natural_fade(world, owner);
            super::summon::despawn_summon(world, actor);
        } else {
            cast_on_owner_target(world, actor);
        }
    }
}

fn wild_imp_exhausted(world: &World, actor: Entity) -> bool {
    let summon = world.get::<SummonedCreature>(actor).unwrap();
    let entry = world.get::<Npc>(actor).unwrap().template_id;
    if (summon.spell_id, entry) != (104317, 55659)
        || world.get::<CastState>(actor).is_some()
        || world
            .resource::<super::missiles::PendingSpellHits>()
            .has_caster(actor)
    {
        return false;
    }
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
    let spells = world.get::<GuardianActor>(actor).unwrap().spells.clone();
    for id in spells {
        let Some(spell) = world.resource::<crate::spell_info::SpellStore>().get(id) else {
            continue;
        };
        let timed = has_repeat_clock(spell);
        let used = world
            .get::<GuardianActor>(actor)
            .unwrap()
            .untimed_used
            .contains(&id);
        if !timed && used {
            continue;
        }
        if super::creature_cast(world, actor, id, Some(target), false).is_err() {
            continue;
        }
        if !timed {
            world
                .get_mut::<GuardianActor>(actor)
                .unwrap()
                .untimed_used
                .push(id);
        }
        break;
    }
}
