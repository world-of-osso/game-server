//! The real Primal Storm Elemental's Stormfury keeps its own channel when the
//! owner-driven guardian target changes mid-channel.
use super::effects::factor_capture::FactorCapture;
use super::script_registry;
use super::summon::{SummonedCreature, load_summon_properties};
use super::test_harness::{Harness, harness, prepare_class_world, spawn_npc, world_db};
use bevy::prelude::*;
use server::networking::CombatTarget;
use shared::components::{Health, Npc};
use shared::protocol::CombatLogKind;

const PRIMAL_STORM: (u32, u32) = (157319, 77942);
const STORMFURY: u32 = 157375;
const STORMFURY_AOE: u32 = 269005;
const CALL_LIGHTNING: u32 = 157348;
const WIND_GUST: u32 = 157331;

fn summoned_storm_elemental() -> (Harness, Entity) {
    let ids = [
        PRIMAL_STORM.0,
        STORMFURY,
        STORMFURY_AOE,
        CALL_LIGHTNING,
        WIND_GUST,
    ];
    let mut h = harness(Vec::new(), &ids, 7).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(prepare_class_world().world_data())
        .insert_resource(load_summon_properties(&conn).unwrap());
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 10_000_000.0,
        max: 10_000_000.0,
    });
    let owner = h.caster;
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        owner,
        PRIMAL_STORM.0,
        Some(owner),
        1
    ));
    for _ in 0..40 {
        h.advance(1);
        if let Some(actor) = storm_elemental(&mut h) {
            return (h, actor);
        }
    }
    panic!("real Primal Storm Elemental within 40 frames");
}

fn storm_elemental(h: &mut Harness) -> Option<Entity> {
    let owner = h.caster;
    h.app
        .world_mut()
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(h.app.world())
        .find(|(_, npc, summon)| npc.template_id == PRIMAL_STORM.1 && summon.owner == owner)
        .map(|(actor, _, _)| actor)
}

fn paid_ticks(h: &Harness, actor: Entity) -> u64 {
    crate::auras::paid_periodic_ticks(h.app.world(), actor, h.npc, STORMFURY)
}

fn advance_until(h: &mut Harness, frames: usize, done: impl Fn(&Harness) -> bool) {
    for _ in 0..frames {
        if done(h) {
            return;
        }
        h.advance(1);
    }
    assert!(done(h), "condition not reached within {frames} frames");
}

fn aoe_hits(h: &Harness, actor: Entity) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(actor.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.spell_id == Some(STORMFURY_AOE)
        })
        .count()
}

#[test]
fn stormfury_keeps_its_channel_tick_after_the_owner_clears_its_target() {
    let (mut h, actor) = summoned_storm_elemental();
    h.select(Some(h.npc));
    advance_until(&mut h, 400, |h| paid_ticks(h, actor) > 0);
    // The guardian copies its owner's selection every update (guardian.rs
    // cast_on_owner_target) while the native channel continues on its target.
    h.select(None);
    advance_until(&mut h, 10, |h| {
        h.app
            .world()
            .get::<CombatTarget>(actor)
            .unwrap()
            .0
            .is_none()
    });
    let ticks = paid_ticks(&h, actor);
    let hits = aoe_hits(&h, actor);
    advance_until(&mut h, 40, |h| paid_ticks(h, actor) > ticks);
    assert!(
        aoe_hits(&h, actor) > hits,
        "the next channel tick still lands"
    );
    // SimC76f8118 sc_shaman.cpp4185–4207: the child uses its parent channel's
    // current tick; native157375 effect1 is +5% per tick.
    let ticks = paid_ticks(&h, actor);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(STORMFURY_AOE)
        .unwrap()
        .clone();
    assert_eq!(
        script_registry::target_damage_multiplier(h.app.world(), actor, h.npc, &spell),
        1.05f32.powf(ticks as f32)
    );
}

#[test]
fn stormfury_recipients_after_a_primary_killed_mid_tick_keep_that_tick() {
    let (mut h, actor) = summoned_storm_elemental();
    let primary = h.npc;
    let secondary = spawn_npc(&mut h.app, 3.0);
    h.app.world_mut().entity_mut(secondary).insert(Health {
        current: 10_000_000.0,
        max: 10_000_000.0,
    });
    h.select(Some(primary));
    advance_until(&mut h, 400, |h| paid_ticks(h, actor) > 0);
    let killing_tick = paid_ticks(&h, actor) + 1;
    h.app
        .world_mut()
        .get_mut::<Health>(primary)
        .unwrap()
        .current = 1.0;
    h.app.insert_resource(FactorCapture::new(
        actor,
        secondary,
        STORMFURY_AOE,
        "stormfury",
        1,
    ));
    advance_until(&mut h, 40, |h| h.health(primary) == 0.0);
    let aoe_on = |target: Entity| {
        move |event: &&shared::protocol::CombatLogEvent| {
            event.spell_id == Some(STORMFURY_AOE) && event.target == Some(target.to_bits())
        }
    };
    let lethal = h
        .inbox
        .log
        .iter()
        .position(|event| aoe_on(primary)(&event) && event.overflow > 0)
        .expect("the killing Stormfury hit on the primary");
    assert!(
        h.inbox.log[lethal..]
            .iter()
            .any(|event| aoe_on(secondary)(&event)),
        "the secondary resolves after the primary's death in the same tick"
    );
    let capture = h.app.world().resource::<FactorCapture>().export();
    let multiplier = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["applied_multipliers"].as_array())
        .flatten()
        .find(|row| row["factor"] == "giantkiller")
        .and_then(|row| row["value"].as_f64())
        .expect("the secondary's Stormfury hit on the killing tick");
    // SimC76f8118 sc_shaman.cpp4204–4207 fixes tick_number before the child executes.
    assert_eq!(multiplier as f32, 1.05f32.powf(killing_tick as f32));
}
