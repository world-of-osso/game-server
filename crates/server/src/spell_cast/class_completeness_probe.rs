//! Opt-in, native-row level-80 cast census; no UDP, database or persistence writes.
//! This records a controlled base-build cast, NOT full rotation/talent parity.
use std::collections::HashSet;
use std::io::Write;

use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use serde::Deserialize;
use serde_json::{Value, json};
use shared::components::{ActiveSpec, Health, Level, Mana, Player, Position};
use shared::protocol::{CombatChannel, SpellCastIntent};

use super::test_harness::*;
use crate::class_progression::KnownSpells;

#[derive(Deserialize)]
struct Manifest {
    specs: Vec<Spec>,
}

#[derive(Deserialize)]
struct Spec {
    id: u32,
    class: u8,
    spells: Vec<Spell>,
}

#[derive(Deserialize)]
struct Spell {
    id: u32,
    passive: bool,
    origin: Vec<String>,
}

fn source_passives(spec: &Spec) -> Vec<u32> {
    spec.spells
        .iter()
        .filter(|spell| {
            spell.passive
                && spell
                    .origin
                    .iter()
                    .any(|origin| !origin.contains("talent") && !origin.contains("race-gated"))
        })
        .map(|spell| spell.id)
        .collect()
}

fn spawn_case(h: &mut Harness, spec: &Spec, spell: &Spell) {
    h.caster = spawn_player(&mut h.app, spec.class, 0.0);
    h.npc = spawn_npc(&mut h.app, 3.0);
    let mut known = source_passives(spec);
    known.push(spell.id);
    h.app.world_mut().entity_mut(h.caster).insert((
        Level(80),
        crate::persistence::ClientId(u64::from(spell.id)),
        shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        ActiveSpec(spec.id),
        KnownSpells::fixture(known),
        Health {
            current: 500_000.0,
            max: 1_000_000.0,
        },
    ));
    let race = match spec.class {
        7 => 11,
        11 | 12 => 4,
        13 => 52,
        _ => 1,
    };
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = race;
    h.app.world_mut().entity_mut(h.npc).insert((
        Level(80),
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
    ));
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(crate::networking::PlayerLink(h.caster));
    h.advance(3);
    fill_resources(h);
    h.inbox = Inbox::default();
}

fn fill_resources(h: &mut Harness) {
    if let Some(mut pools) = h
        .app
        .world_mut()
        .get_mut::<super::powers::PowerPools>(h.caster)
    {
        for pool in &mut pools.pools {
            pool.current = pool.max;
        }
    }
    let mut mana = h.app.world_mut().get_mut::<Mana>(h.caster).unwrap();
    mana.current = mana.max;
}

fn resources(h: &Harness) -> Value {
    let powers: Vec<_> = h.app.world().get::<super::powers::PowerPools>(h.caster)
        .into_iter().flat_map(|pools| &pools.pools)
        .map(|pool| json!({"power": format!("{:?}", pool.power), "current": pool.current, "max": pool.max}))
        .collect();
    let mana = h.app.world().get::<Mana>(h.caster).unwrap();
    json!({"pools": powers, "mana": mana.current})
}

fn choose_case_target(h: &mut Harness, spell: &crate::spell_info::SpellInfo) -> (Entity, bool) {
    let resurrects = spell
        .effects
        .iter()
        .any(|effect| matches!(effect.effect, 18 | 172));
    let relation = super::scripts::target_plan(spell.id)
        .map(|(relation, _)| relation)
        .or_else(|| super::targets::explicit_requirement(super::handled_effects(spell)))
        .or_else(|| resurrects.then_some(super::targets::Relation::Ally));
    let hostile = relation == Some(super::targets::Relation::Enemy)
        || (relation == Some(super::targets::Relation::Any)
            && (spell.id == 361469 || spell.effects.iter().any(|effect| effect.effect == 2)));
    let target = if resurrects {
        let ally = spawn_player(&mut h.app, 1, 3.0);
        h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 0.0;
        ally
    } else if hostile {
        h.npc
    } else {
        h.caster
    };
    (target, hostile)
}

fn send_case_cast(
    h: &mut Harness,
    spell: &crate::spell_info::SpellInfo,
    target: Entity,
    hostile: bool,
) {
    let (min, max) = super::targets::spell_range(spell, hostile);
    let distance = (min + 0.5).max(3.0).min(max.max(3.0));
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: distance,
        y: 0.0,
        z: 0.0,
    });
    if super::attributes(spell, 0) & super::SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL != 0 {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(shared::components::CombatStatus(false));
    }
    let destination = spell
        .target_restrictions
        .as_ref()
        .filter(|restriction| restriction.targets & 0x40 != 0)
        .map(|_| [distance, 0.0, 0.0]); // Native TARGET_FLAG_DEST_LOCATION.
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(spell.id),
            spell: String::new(),
            target_entity: Some(target.to_bits()),
            target_item_guid: None,
            witness: None,
            destination,
        });
}

fn cast_case(h: &mut Harness, spell: &crate::spell_info::SpellInfo) {
    let (target, hostile) = choose_case_target(h, spell);
    send_case_cast(h, spell, target, hostile);
    h.advance(3);
    let cast_ms = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.cast_time)
        .map_or(0, |cast| cast.base_ms.max(0) as usize);
    h.advance(cast_ms / 50 + 200); // Native cast time plus ten seconds of ticks/flight.
}

fn record_case(h: &mut Harness, spec: &Spec, row: &Spell) -> Value {
    let Some(spell) = world_db().unwrap().store.get(row.id).cloned() else {
        return json!({"spec": spec.id, "spell": row.id, "missing_store": true});
    };
    spawn_case(h, spec, row);
    let baseline_warnings = crate::auras::take_unhandled_auras(h.app.world_mut());
    h.app
        .world_mut()
        .resource_mut::<super::effects::UnhandledEffects>()
        .0
        .clear();
    let before = resources(h);
    if !row.passive {
        cast_case(h, &spell);
    }
    let after = resources(h);
    let warnings = crate::auras::take_unhandled_auras(h.app.world_mut());
    let effects: Vec<_> = h
        .app
        .world()
        .resource::<super::effects::UnhandledEffects>()
        .0
        .iter()
        .copied()
        .collect();
    let applied = crate::auras::has_aura(h.app.world(), h.caster, row.id);
    json!({"spec": spec.id, "spell": row.id, "passive": row.passive,
        "passive_applied": applied, "failed": h.inbox.failed, "go": h.inbox.go,
        "baseline_aura_warnings": baseline_warnings, "aura_warnings": warnings,
        "effect_warnings": effects, "resources_before": before, "resources_after": after,
        "combat_log": h.inbox.log})
}

fn despawn_case(h: &mut Harness, retained: &HashSet<Entity>) {
    let entities: Vec<_> = h
        .app
        .world_mut()
        .query::<Entity>()
        .iter(h.app.world())
        .filter(|entity| !retained.contains(entity))
        .collect();
    for entity in entities {
        h.app.world_mut().despawn(entity);
    }
}

fn census_harness(class: u8) -> Harness {
    let data = world_db().unwrap();
    let mut h = harness(Vec::new(), &[], class).unwrap();
    h.app.insert_resource(data.store.clone());
    h.app
        .insert_resource(crate::world_data::load_world_data(&data.powers_db));
    // Cargo executes tests from the crate directory; production's relative
    // data/world.db startup path is not this fixture's authored database.
    let conn = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::teleport_positions::load_positions(&conn).unwrap());
    h.app
        .insert_resource(crate::maps::store::load_map_store(&conn).unwrap());
    h.app.init_resource::<crate::maps::InstanceManager>();
    h
}

#[test]
fn class_completeness_probe_native_city_destination_is_not_a_fixture_failure() {
    let mut h = census_harness(8);
    let spec = Spec {
        id: 62,
        class: 8,
        spells: Vec::new(),
    };
    let spell = Spell {
        id: 3567,
        passive: false,
        origin: vec!["SkillLineAbility".into()],
    };
    let receipt = record_case(&mut h, &spec, &spell);
    assert_eq!(receipt["failed"], json!([]), "native city cast: {receipt}");
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let destinations = super::teleport_positions::load_positions(&conn).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::maps::MapTransfer>(h.caster)
            .unwrap()
            .destination,
        destinations.0[&(3567, 0)],
    );
}

#[test]
#[ignore = "explicit read-only census with manifest/output paths; not a full suite"]
fn class_completeness_level80_probe() {
    let input = std::env::var("CLASS_COMPLETENESS_MANIFEST").expect("manifest path");
    let output = std::env::var("CLASS_COMPLETENESS_OUTPUT").expect("output path");
    let manifest: Manifest =
        serde_json::from_str(&std::fs::read_to_string(input).unwrap()).unwrap();
    let mut file = std::fs::File::create(output).unwrap();
    let mut records = 0;
    for spec in &manifest.specs {
        let mut h = census_harness(spec.class);
        let retained: HashSet<_> = h
            .app
            .world_mut()
            .query::<Entity>()
            .iter(h.app.world())
            .filter(|entity| *entity != h.caster && *entity != h.npc)
            .collect();
        despawn_case(&mut h, &retained);
        for spell in &spec.spells {
            let record = record_case(&mut h, spec, spell);
            writeln!(file, "{record}").unwrap();
            despawn_case(&mut h, &retained);
            records += 1;
        }
        file.flush().unwrap();
        println!(
            "spec {}: {} native spell cases recorded",
            spec.id,
            spec.spells.len()
        );
    }
    assert_eq!(
        records,
        manifest
            .specs
            .iter()
            .map(|spec| spec.spells.len())
            .sum::<usize>()
    );
}
