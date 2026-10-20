//! Native70448 Abundance is threshold5/single-stack, not old TC per-HoT stacks.
use super::kit_tests::at_level;
use super::{FollowerCatalog, spawning};
use crate::auras::{ActiveAuras, aura_now, has_aura};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use crate::spell_cast::{SpellHistory, creature_cast, history_now};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

const CRENNA: u32 = 209072;
const TALENT: u32 = 207383;
const ABUNDANCE: u32 = 207640;
const REJUVENATION: u32 = 774;
const FRIENDS: &[u32] = &[209057, 209059, 209065];

fn add_native_friend(h: &mut Harness, entry: u32) -> Entity {
    let profile = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .profiles
        .iter()
        .find(|profile| profile.entry == entry)
        .unwrap()
        .clone();
    let unit = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(shared::components::Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        });
    let owner = h.caster;
    spawning::initialize(h.app.world_mut(), owner, unit, profile).unwrap();
    unit
}

fn native_party(level: u8) -> (Harness, Entity, [Entity; 5]) {
    let (mut h, healer) = at_level(CRENNA, level, |_| {});
    let native = crate::spell_cast::test_harness::load_native_creature_world(FRIENDS);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    let owner = h.caster;
    let patients = [
        healer,
        owner,
        add_native_friend(&mut h, FRIENDS[0]),
        add_native_friend(&mut h, FRIENDS[1]),
        add_native_friend(&mut h, FRIENDS[2]),
    ];
    let spell = h.app.world().resource::<SpellStore>().get(TALENT).unwrap();
    let threshold = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(threshold, 5.0);
    assert_eq!(patients.len(), threshold as usize);
    (h, healer, patients)
}

fn own_rejuvenation_expiry(h: &Harness, caster: Entity, target: Entity) -> Option<f64> {
    h.app
        .world()
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == REJUVENATION)?
        .expires_at
}

fn own_count(h: &Harness, caster: Entity, patients: &[Entity]) -> usize {
    patients
        .iter()
        .filter(|target| own_rejuvenation_expiry(h, caster, **target).is_some())
        .count()
}

fn cast_real_rejuvenation(h: &mut Harness, caster: Entity, target: Entity) {
    creature_cast(h.app.world_mut(), caster, REJUVENATION, Some(target), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(REJUVENATION).unwrap();
        let locked = world
            .get::<SpellHistory>(caster)
            .is_some_and(|history| history.on_gcd(spell, history_now(world)));
        if !locked {
            assert!(own_rejuvenation_expiry(h, caster, target).is_some());
            return;
        }
    }
    panic!("ordinary source Rejuvenation GCD did not clear");
}

fn prepare_five_native_hots() -> (Harness, Entity, [Entity; 5]) {
    let (mut h, healer, patients) = native_party(69);
    assert!(has_aura(h.app.world(), healer, TALENT));
    for patient in patients {
        cast_real_rejuvenation(&mut h, healer, patient);
    }
    assert_eq!(own_count(&h, healer, &patients), patients.len());
    assert!(
        has_aura(h.app.world(), healer, ABUNDANCE),
        "actual learned69 plus5 own primary774s must grant207640"
    );
    (h, healer, patients)
}

#[test]
#[ignore = "ABUNDANCE_NATIVE_THRESHOLD_5: level68 absent-talent control"]
fn follower_abundance_level68_five_native_rejuvenations_do_not_grant_buff() {
    let (mut h, healer, patients) = native_party(68);
    assert!(!has_aura(h.app.world(), healer, TALENT));
    for patient in patients {
        cast_real_rejuvenation(&mut h, healer, patient);
    }
    assert_eq!(own_count(&h, healer, &patients), patients.len());
    assert!(!has_aura(h.app.world(), healer, ABUNDANCE));
}

#[test]
#[ignore = "ABUNDANCE_NATIVE_THRESHOLD_5: four-to-five own primary HoTs"]
fn follower_abundance_level69_fifth_native_rejuvenation_grants_one_stack() {
    let (mut h, healer, patients) = native_party(69);
    assert!(has_aura(h.app.world(), healer, TALENT));
    for &patient in &patients[..4] {
        cast_real_rejuvenation(&mut h, healer, patient);
    }
    assert_eq!(own_count(&h, healer, &patients), 4);
    assert!(!has_aura(h.app.world(), healer, ABUNDANCE));
    cast_real_rejuvenation(&mut h, healer, patients[4]);
    assert_eq!(own_count(&h, healer, &patients), 5);
    let auras = h.app.world().get::<ActiveAuras>(healer).unwrap();
    let buff = auras
        .iter()
        .find(|aura| aura.spell_id == ABUNDANCE)
        .expect("fifth own Rejuvenation must grant native207640");
    assert_eq!(
        buff.stacks, 1,
        "70448 Abundance is one threshold buff, not per-HoT stacking"
    );
}

#[test]
#[ignore = "ABUNDANCE_NATIVE_THRESHOLD_5: five-HoT prerequisite before natural expiry removal"]
fn follower_abundance_level69_first_native_expiry_removes_threshold_buff() {
    let (mut h, healer, patients) = prepare_five_native_hots();
    let deadline = patients
        .iter()
        .map(|target| own_rejuvenation_expiry(&h, healer, *target).unwrap())
        .min_by(f64::total_cmp)
        .unwrap();
    let remaining = deadline - aura_now(h.app.world());
    assert!(remaining > 0.0);
    let frame = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(frame) => frame.as_secs_f64(),
        _ => panic!("native qualification needs a deterministic frame duration"),
    };
    h.advance((remaining / frame).ceil() as usize + 1);
    assert_eq!(
        own_count(&h, healer, &patients),
        4,
        "only the naturally earliest source HoT should have expired"
    );
    assert!(
        !has_aura(h.app.world(), healer, ABUNDANCE),
        "dropping below the native threshold must remove207640"
    );
}
