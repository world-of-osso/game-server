//! TC a352b1fa spell_shaman.cpp1006-1034: native5%, once per target per field.
use super::test_harness::{
    Harness, ObservedTriggeredCasts, harness, spawn_npc, spawn_player, world_db,
};
use bevy::prelude::*;
use std::collections::HashMap;

const EARTHQUAKE: u32 = 61882;
const TARGET_EARTHQUAKE: u32 = 462620;
const TICK: u32 = 77478;
const KNOCKDOWN: u32 = 77505;
const CENTER: Vec3 = Vec3::new(20.0, 0.0, 0.0);

fn fixture() -> Harness {
    let mut h = harness(Vec::new(), &[EARTHQUAKE, TICK, KNOCKDOWN], 7).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    h.app.insert_resource(ObservedTriggeredCasts::default());
    h
}

fn knockdowns(h: &Harness) -> HashMap<Entity, usize> {
    let mut counts = HashMap::new();
    for &(caster, spell, target) in &h.app.world().resource::<ObservedTriggeredCasts>().0 {
        if spell == KNOCKDOWN {
            assert_eq!(caster, h.caster);
            *counts
                .entry(target.expect("actual knockdown target"))
                .or_default() += 1;
        }
    }
    counts
}

#[test]
fn census_e77_shaman_native_knockdown_is_once_per_target_per_owned_field() {
    let mut h = fixture();
    //256 authentic targets × six native5% rolls: absence probability <1e-30.
    //No RNG edits, synthetic chance, health writes, fabricated rows or manual clocks.
    let victims: Vec<_> = (0..256).map(|_| spawn_npc(&mut h.app, CENTER.x)).collect();
    let far = spawn_npc(&mut h.app, 40.0);
    let ally = spawn_player(&mut h.app, 7, CENTER.x);
    super::cast_triggered_at_destination(h.app.world_mut(), h.caster, EARTHQUAKE, CENTER, 0);
    h.advance(130);
    let first = knockdowns(&h);
    assert!(
        !first.is_empty(),
        "actual native field must cast77505 from E1 chance"
    );
    assert!(first.keys().all(|target| victims.contains(target)));
    assert!(
        first.values().all(|&count| count == 1),
        "one stun per field:{first:?}"
    );
    assert_eq!(h.health(far), 5000.0);
    assert_eq!(h.health(ally), 1000.0);
    assert!(victims.iter().all(|&target| h.health(target) < 5000.0));
    super::cast_triggered_at_destination(h.app.world_mut(), h.caster, EARTHQUAKE, CENTER, 0);
    h.advance(130);
    let second = knockdowns(&h);
    assert!(
        second.values().all(|&count| count <= 2),
        "new field resets only its own set"
    );
    assert!(
        second.values().any(|&count| count == 2),
        "fresh field can stun a prior target"
    );
    assert!(
        !h.app
            .world()
            .resource::<super::effects::UnhandledEffects>()
            .0
            .contains(&(TICK, 77)),
        "native E1 chance is consumed data, not unsupported dispatch"
    );
    let before = second;
    h.advance(80);
    assert_eq!(
        knockdowns(&h),
        before,
        "no stun after authored field expiry"
    );
}

#[test]
fn census_e77_shaman_target_earthquake_preserves_native_center_overlap_and_expiry() {
    let mut h = fixture();
    let target = spawn_npc(&mut h.app, CENTER.x);
    let ally = spawn_player(&mut h.app, 7, CENTER.x);
    let first = h.inbox.log.len();
    super::cast_triggered(
        h.app.world_mut(),
        h.caster,
        TARGET_EARTHQUAKE,
        Some(target),
        0,
    );
    h.advance(20);
    super::cast_triggered(
        h.app.world_mut(),
        h.caster,
        TARGET_EARTHQUAKE,
        Some(target),
        0,
    );
    h.advance(140);
    let hits: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.spell_id == Some(TICK)
                && event.source == Some(h.caster.to_bits())
        })
        .collect();
    assert_eq!(
        hits.len(),
        12,
        "two actual native6s/1000ms fields overlap without replacing one another"
    );
    assert!(
        hits.iter()
            .all(|event| event.target == Some(target.to_bits()) && event.amount > 0)
    );
    assert_eq!(
        h.health(h.npc),
        5000.0,
        "target-centered8yd field does not move to caster or its10yd original target"
    );
    assert_eq!(h.health(ally), 1000.0);
    let first = h.inbox.log.len();
    h.advance(40);
    assert!(
        h.inbox.log[first..]
            .iter()
            .all(|event| event.spell_id != Some(TICK)),
        "natural field expiry stops damage"
    );
}

#[test]
fn census_e77_shaman_orphan_native_tick_deals_damage_without_knockdown() {
    let mut h = fixture();
    let victim = spawn_npc(&mut h.app, CENTER.x);
    for _ in 0..32 {
        super::cast_triggered_at_destination(h.app.world_mut(), h.caster, TICK, CENTER, 0);
        h.advance(1);
    }
    assert!(h.health(victim) < 5000.0);
    assert!(
        knockdowns(&h).is_empty(),
        "no field GUID means no per-field knockdown"
    );
}
