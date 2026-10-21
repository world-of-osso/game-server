//! Real request clocks for explicitly opted current-Retail LittleWigs scripts.
//! No SmartAI inference. Warning colors use the existing bossmod importer policy:
//! red=High, orange/purple=Medium, yellow=Low. Missing dynamic text stays absent.
use std::collections::HashMap;

use bevy::prelude::*;
use shared::protocol::{
    CombatLogEvent, CombatLogKind, EncounterSeverity, EncounterTimelineEvent, EncounterTimer,
};

use super::{EventMap, instance_script};
use crate::{maps::InMap, spell_info::SpellStore};

#[derive(Clone, Copy)]
struct Ability {
    spell: u32,
    label: u32,
    warning: Option<EncounterSeverity>,
    counted: bool,
    warning_name: Option<&'static str>,
}

const fn ability(
    spell: u32,
    label: u32,
    warning: Option<EncounterSeverity>,
    counted: bool,
) -> Ability {
    Ability {
        spell,
        label,
        warning,
        counted,
        warning_name: None,
    }
}

const fn counted_rename(spell: u32, name: &'static str, warning: EncounterSeverity) -> Ability {
    Ability {
        warning_name: Some(name),
        ..ability(spell, spell, Some(warning), true)
    }
}

/// LittleWigs2777a9d: current Retail timeline callbacks for Ruby/Academy,
/// ordinary cast handlers for Rookery/Azure Vault. IDs are explicit native
/// requests, not every spell on a boss or an inferred event-id→spell cast.
fn abilities(encounter: u32) -> Option<&'static [Ability]> {
    use EncounterSeverity::{High, Low, Medium};
    Some(match encounter {
        // KokiaBlazehoof.lua:208–243. Ritual needs absent Blizzard target text.
        2606 => &[
            ability(372864, 372864, None, true),
            ability(372107, 372110, Some(Medium), true),
            ability(372858, 372858, Some(Medium), true),
        ],
        // MelidrussaChillworn.lua:239–276; wrappers name native children.
        2609 => &[
            ability(396044, 1307297, Some(Medium), true),
            ability(1311543, 1307308, Some(Low), true),
        ],
        // Kyrioss.lua:70–110:1214315 starts a castbar, not the444034 warning.
        2816 => &[
            ability(1214325, 1214325, Some(High), false),
            ability(474018, 474018, Some(Medium), false),
            ability(1214315, 444123, None, false),
        ],
        // EchoOfDoragosa.lua:184–245.
        2565 => &[
            counted_rename(373326, "Missiles", Medium),
            counted_rename(1282251, "Tank Hit", Medium),
            counted_rename(374343, "Bombs", Low),
            counted_rename(388822, "Pull In", High),
        ],
        // Azureblade.lua:108–140.
        2585 => &[
            ability(384223, 384223, Some(Low), false),
            ability(372222, 372222, Some(Medium), false),
            ability(385578, 385578, Some(High), false),
        ],
        // Umbrelskul.lua:103–137; role/target-dependent Strike text not fabricated.
        2584 => &[
            ability(384978, 384978, None, false),
            ability(384699, 384699, Some(High), false),
        ],
        _ => return None,
    })
}

pub(super) fn owns_clock(encounter: u32) -> bool {
    abilities(encounter).is_some()
}

#[derive(Component, Default)]
struct Publication {
    signature: Vec<(u32, u64)>,
    sampled_at: f64,
    counts: HashMap<u32, u32>,
}

fn project_schedule(
    events: &EventMap,
    abilities: &[Ability],
) -> (Vec<(u32, u64)>, Vec<EncounterTimer>) {
    let selected: Vec<_> = events
        .pending_events()
        .filter_map(|(event, due, remaining, duration)| {
            let ability = abilities.iter().find(|ability| ability.spell == event)?;
            Some((
                (event, due),
                EncounterTimer {
                    event_id: event,
                    spell_id: ability.label,
                    remaining_ms: remaining.min(u64::from(u32::MAX)) as u32,
                    duration_ms: duration.min(u64::from(u32::MAX)) as u32,
                    conditional: true,
                },
            ))
        })
        .collect();
    selected.into_iter().unzip()
}

pub(super) fn publish_schedule(world: &mut World, unit: Entity, events: &EventMap) {
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let Some(encounter) = instance_script::active_encounter_id(world, map, unit) else {
        world.entity_mut(unit).remove::<Publication>();
        return;
    };
    let Some(abilities) = abilities(encounter) else {
        return;
    };
    let (signature, timers) = project_schedule(events, abilities);
    let now = world.resource::<Time>().elapsed_secs_f64();
    {
        let mut entity = world.entity_mut(unit);
        let mut publication = entity.entry::<Publication>().or_default().into_mut();
        if signature == publication.signature && now - publication.sampled_at < 1.0 {
            return;
        }
        publication.signature = signature;
        publication.sampled_at = now;
    }
    instance_script::send_to_players(
        world,
        map,
        EncounterTimelineEvent::Timers {
            caster: unit.to_bits(),
            sample_unix_ms: shared::protocol::read_encounter_unix_ms(),
            timers,
        },
    );
}

fn select_cast_warning(
    world: &World,
    event: &CombatLogEvent,
) -> Option<(Entity, InMap, Ability, String)> {
    if !matches!(
        event.kind,
        CombatLogKind::CastStart | CombatLogKind::CastSuccess
    ) {
        return None;
    }
    let unit = event.source.and_then(Entity::try_from_bits)?;
    let map = *world.get::<InMap>(unit)?;
    let encounter = instance_script::active_encounter_id(world, map, unit)?;
    let abilities = abilities(encounter)?;
    let ability = *abilities
        .iter()
        .find(|ability| Some(ability.spell) == event.spell_id)?;
    ability.warning?;
    let store = world.resource::<SpellStore>();
    let spell = store
        .get(ability.spell)
        .expect("sourced native encounter spell");
    let starts_cast = crate::spell_cast::cast_time_ms(world, unit, spell) > 0;
    let boundary = if starts_cast {
        CombatLogKind::CastStart
    } else {
        CombatLogKind::CastSuccess
    };
    if event.kind != boundary {
        return None;
    }
    let name = match ability.warning_name {
        Some(name) => name.to_owned(),
        None => store
            .get(ability.label)
            .expect("sourced encounter label")
            .name
            .clone(),
    };
    Some((unit, map, ability, name))
}

pub(super) fn cast_event(world: &mut World, event: &CombatLogEvent) {
    let Some((unit, map, ability, name)) = select_cast_warning(world, event) else {
        return;
    };
    let severity = ability.warning.expect("selected warning severity");
    let Some(mut publication) = world.get_mut::<Publication>(unit) else {
        return;
    };
    let count = publication.counts.entry(ability.spell).or_default();
    *count += 1;
    let text = if ability.counted {
        format!("{name} ({count})")
    } else {
        name
    };
    instance_script::send_to_players(
        world,
        map,
        EncounterTimelineEvent::Cast {
            caster: unit.to_bits(),
            spell_id: ability.label,
            severity,
            warning_text: text,
        },
    );
}

pub(super) fn disengage(world: &mut World, unit: Entity) {
    world.entity_mut(unit).remove::<Publication>();
}
