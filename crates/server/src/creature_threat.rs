//! Server-owned threat selection; TC a352b1fa ThreatManager.cpp:581–657.
use super::*;

/// ThreatManager's victim is distinct from the creature's active attack/chase target.
#[derive(Component, Default, Clone, Copy)]
pub(crate) struct CurrentVictim {
    pub(crate) target: Option<Entity>,
    next_update: f64,
}

/// ThreatManager.h:86, ThreatManager.cpp:216–230.
const UPDATE_SECONDS: f64 = 1.0;
const MELEE_THRESHOLD: f32 = 1.1;
const RANGED_THRESHOLD: f32 = 1.3;
const MELEE_REACH_OFFSET: f32 = 4.0 / 3.0;
const NOMINAL_MELEE_RANGE: f32 = 5.0;

struct Candidate {
    unit: Entity,
    threat: f32,
    in_melee: bool,
}

/// TC ThreatManager.cpp:242–248: refresh an absent/offline reference lazily.
/// A query changes selection, never the creature's active attack/chase projection.
pub(crate) fn current_victim(world: &mut World, creature: Entity) -> Option<Entity> {
    let threat = world.get::<ThreatTable>(creature)?;
    let current = world
        .get::<CurrentVictim>(creature)
        .and_then(|reference| reference.target);
    let valid = current.is_some_and(|unit| {
        threat
            .entries()
            .iter()
            .any(|entry| entry.entity == unit.to_bits())
            && available(world, creature, unit)
    });
    if !valid {
        refresh_reference(world, creature);
    }
    world.get::<CurrentVictim>(creature)?.target
}

pub(super) fn update_reference_when_due(world: &mut World, creature: Entity) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world
        .get::<CurrentVictim>(creature)
        .is_some_and(|reference| now < reference.next_update)
    {
        return;
    }
    refresh_reference(world, creature);
    world
        .get_mut::<CurrentVictim>(creature)
        .unwrap()
        .next_update = now + UPDATE_SECONDS;
}

fn refresh_reference(world: &mut World, creature: Entity) {
    let previous = world
        .get::<CurrentVictim>(creature)
        .copied()
        .unwrap_or_default();
    let candidates = collect_candidates(world, creature);
    let fixate = world.get::<Fixate>(creature).map(|fixate| fixate.0);
    let fixate_listed = fixate.is_none_or(|unit| {
        world.get::<ThreatTable>(creature).is_some_and(|threat| {
            threat
                .entries()
                .iter()
                .any(|entry| entry.entity == unit.to_bits())
        })
    });
    if !fixate_listed {
        world.entity_mut(creature).remove::<Fixate>();
    }
    let taunted = latest_available_taunt(world, creature, &candidates);
    let target = select_victim(&candidates, previous.target, fixate, taunted);
    world
        .entity_mut(creature)
        .insert(CurrentVictim { target, ..previous });
}

fn collect_candidates(world: &World, creature: Entity) -> Vec<Candidate> {
    let Some(threat) = world.get::<ThreatTable>(creature) else {
        return Vec::new();
    };
    let mut candidates: Vec<_> = threat
        .entries()
        .iter()
        .filter_map(|entry| {
            let unit = Entity::try_from_bits(entry.entity)?;
            available(world, creature, unit).then(|| Candidate {
                unit,
                threat: (entry.threat + temporary_threat(world, unit)).max(0.0),
                in_melee: in_melee_range(world, creature, unit),
            })
        })
        .collect();
    candidates.sort_by(|left, right| right.threat.total_cmp(&left.threat));
    candidates
}

/// TC ThreatManager.cpp773-790: reversible flat modifier, never base threat.
fn temporary_threat(world: &World, unit: Entity) -> f32 {
    if world.get::<Player>(unit).is_none() {
        return 0.0;
    }
    world
        .get::<crate::auras::AuraStatMods>(unit)
        .map_or(0.0, |mods| mods.temporary_threat)
}

pub(crate) fn refresh_temporary_threat(world: &mut World, unit: Entity) {
    let affected: Vec<Entity> = world
        .query::<(Entity, &ThreatTable)>()
        .iter(world)
        .filter(|(_, table)| {
            table
                .entries()
                .iter()
                .any(|entry| entry.entity == unit.to_bits())
        })
        .map(|(creature, _)| creature)
        .collect();
    for creature in affected {
        refresh_reference(world, creature);
    }
}

fn latest_available_taunt(
    world: &World,
    creature: Entity,
    candidates: &[Candidate],
) -> Option<Entity> {
    world
        .get::<crate::auras::ActiveAuras>(creature)?
        .iter()
        .filter(|aura| {
            aura.effects
                .iter()
                .any(|effect| effect.aura_type == crate::auras::handlers::MOD_TAUNT)
                && candidates
                    .iter()
                    .any(|candidate| candidate.unit == aura.caster)
        })
        .last()
        .map(|aura| aura.caster)
}

fn select_victim(
    candidates: &[Candidate],
    incumbent: Option<Entity>,
    fixate: Option<Entity>,
    taunted: Option<Entity>,
) -> Option<Entity> {
    if let Some(fixated) =
        fixate.filter(|unit| candidates.iter().any(|candidate| candidate.unit == *unit))
    {
        return Some(fixated);
    }
    if taunted.is_some() {
        return taunted;
    }
    select_ranked_challenger(candidates, incumbent)
}

fn select_ranked_challenger(candidates: &[Candidate], incumbent: Option<Entity>) -> Option<Entity> {
    let highest = candidates.first()?;
    let Some(current) = candidates
        .iter()
        .find(|candidate| Some(candidate.unit) == incumbent)
    else {
        return Some(highest.unit);
    };
    // TC:620–626,674–681: exact110%/130% retains the incumbent.
    if highest.unit == current.unit || highest.threat <= current.threat * MELEE_THRESHOLD {
        return Some(current.unit);
    }
    if highest.threat > current.threat * RANGED_THRESHOLD {
        return Some(highest.unit);
    }
    // TC:629–657: a lower-ranked melee challenger can beat the ranged highest.
    candidates
        .iter()
        .take_while(|candidate| candidate.threat > current.threat * MELEE_THRESHOLD)
        .find(|candidate| candidate.in_melee)
        .map_or(Some(current.unit), |candidate| Some(candidate.unit))
}

/// ThreatReference::ShouldBeOffline; unavailable references retain their threat.
pub(super) fn available(world: &World, creature: Entity, unit: Entity) -> bool {
    let Some(map) = world.get::<InMap>(creature) else {
        return false;
    };
    if !living_target(world, unit) || world.get::<InMap>(unit) != Some(map) {
        return false;
    }
    if world.get::<TaxiFlight>(unit).is_some() {
        return false;
    }
    if world
        .get::<UnitFlags>(unit)
        .is_some_and(|flags| !flags.can_be_attacked_by(false))
    {
        return false;
    }
    let factions = world.resource::<FactionTemplates>();
    if !factions.can_attack(unit_faction(world, creature), unit_faction(world, unit)) {
        return false;
    }
    crate::auras::stealth::can_detect(world, creature, unit)
        && within_home_reach(world, creature, unit)
}

fn living_target(world: &World, unit: Entity) -> bool {
    crate::spell_cast::targets::is_alive(world, unit)
        && world.get::<DeathState>(unit).is_none()
        && world
            .get::<PlayerDeathState>(unit)
            .is_none_or(|death| death.state == shared::death::DeathState::Alive)
}

fn unit_faction(world: &World, unit: Entity) -> UnitFaction {
    UnitFaction {
        template: world
            .get::<UnitFactionTemplate>(unit)
            .map_or(0, |faction| faction.0),
        is_player: crate::faction::player_affected(
            world.get::<Player>(unit).is_some(),
            world.get::<UnitFlags>(unit),
        ),
    }
}

fn within_home_reach(world: &World, creature: Entity, unit: Entity) -> bool {
    let Some(position) = world.get::<Position>(unit) else {
        return false;
    };
    let dungeon = world
        .get_resource::<crate::maps::MapStore>()
        .zip(world.get::<InMap>(creature))
        .and_then(|(maps, map)| maps.map(map.map_id))
        .is_some_and(|map| map.is_dungeon());
    within_reach_of_home(
        world.get::<SpawnPoint>(creature),
        world.get::<LastDamagedTime>(creature),
        world.resource::<Time>().elapsed_secs_f64(),
        dungeon,
        position,
    )
}

fn in_melee_range(world: &World, creature: Entity, unit: Entity) -> bool {
    let reach = |entity| crate::scripted_ai::model_reach::combat_reach(world, entity);
    let Some((own, victim)) = reach(creature).zip(reach(unit)) else {
        error!(
            ?creature,
            ?unit,
            "Threat reselection requires sourced combat reaches"
        );
        return false;
    };
    let Some((from, to)) = world
        .get::<Position>(creature)
        .zip(world.get::<Position>(unit))
    else {
        return false;
    };
    // Unit.cpp:691–710: IsWithinMeleeRangeAt/GetMeleeRange, inclusive distance.
    let base_range = (own + victim + MELEE_REACH_OFFSET).max(NOMINAL_MELEE_RANGE);
    let bonus = world
        .get::<crate::auras::AuraStatMods>(creature)
        .map_or(0.0, |mods| mods.autoattack_range_bonus);
    distance(from, to) <= base_range + bonus
}
