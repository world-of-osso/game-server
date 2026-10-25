//! Native70448 aura-world slice: no altered spells or fabricated effect amounts.
use super::world_state::{AuraTeleportOrigins, WorldAuraState};
use super::*;
use crate::spell_cast::test_harness::{Harness, harness};
use shared::components::{Position, PowerType, Rotation};

pub(crate) fn apply_native(world: &mut World, unit: Entity, spell_id: u32) {
    let mask = aura_effect_mask(world.resource::<SpellStore>().get(spell_id).unwrap());
    apply_aura(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

pub(crate) fn native_auras(id: u32) -> ActiveAuras {
    let mut h = setup(&[id], 1);
    apply_native(h.app.world_mut(), h.caster, id);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .take::<ActiveAuras>()
        .unwrap()
}

fn setup(ids: &[u32], class: u8) -> Harness {
    harness(Vec::new(), ids, class).expect("native70448 world.db")
}

fn clear(h: &mut Harness, spell: u32) {
    remove_auras(h.app.world_mut(), h.caster, RemoveReason::Cancelled, |a| {
        a.spell_id == spell
    });
}

macro_rules! movement_flag {
    ($name:ident, $spell:expr, $field:ident) => {
        #[test]
        fn $name() {
            let mut h = setup(&[$spell], 1);
            apply_native(h.app.world_mut(), h.caster, $spell);
            assert!(
                h.app
                    .world()
                    .get::<WorldAuraState>(h.caster)
                    .is_some_and(|s| s.$field)
            );
            clear(&mut h, $spell);
            assert!(
                !h.app
                    .world()
                    .get::<WorldAuraState>(h.caster)
                    .unwrap()
                    .$field
            );
        }
    };
}
movement_flag!(aura_world_water_walk_546, 546, water_walk);
movement_flag!(aura_world_hover_11010, 11010, hover);
movement_flag!(aura_world_disable_gravity_195289, 195289, disable_gravity);

#[test]
fn aura_world_water_walk_overlapping_sources_survive_one_removal() {
    let mut h = setup(&[546, 3714], 1);
    for spell in [546, 3714] {
        apply_native(h.app.world_mut(), h.caster, spell);
    }
    clear(&mut h, 546);
    assert!(
        h.app
            .world()
            .get::<WorldAuraState>(h.caster)
            .is_some_and(|s| s.water_walk)
    );
    clear(&mut h, 3714);
    assert!(
        !h.app
            .world()
            .get::<WorldAuraState>(h.caster)
            .unwrap()
            .water_walk
    );
}

#[test]
fn aura_world_tracking_beasts_humanoids_player_only() {
    let mut h = setup(&[1494, 5225], 1);
    for spell in [1494, 5225] {
        apply_native(h.app.world_mut(), h.caster, spell);
    }
    assert_eq!(
        h.app
            .world()
            .get::<WorldAuraState>(h.caster)
            .map(|s| s.tracked_creatures),
        Some(0x41)
    );
    clear(&mut h, 1494);
    assert_eq!(
        h.app
            .world()
            .get::<WorldAuraState>(h.caster)
            .unwrap()
            .tracked_creatures,
        0x40
    );
    apply_native(h.app.world_mut(), h.npc, 1494);
    assert_eq!(
        h.app
            .world()
            .get::<WorldAuraState>(h.npc)
            .unwrap()
            .tracked_creatures,
        0
    );
}

#[test]
fn aura_world_force_magnitude_multiplies_and_restores() {
    let mut h = setup(&[188494, 189425], 1);
    apply_native(h.app.world_mut(), h.caster, 188494); // -50%
    apply_native(h.app.world_mut(), h.caster, 189425); // -60%
    let state = h
        .app
        .world()
        .get::<WorldAuraState>(h.caster)
        .expect("movement force state");
    assert!((state.movement_force_multiplier - 0.2).abs() < 0.0001);
    clear(&mut h, 188494);
    assert!(
        (h.app
            .world()
            .get::<WorldAuraState>(h.caster)
            .unwrap()
            .movement_force_multiplier
            - 0.4)
            .abs()
            < 0.0001
    );
    clear(&mut h, 189425);
    assert_eq!(
        h.app
            .world()
            .get::<WorldAuraState>(h.caster)
            .unwrap()
            .movement_force_multiplier,
        1.0
    );
}

#[test]
fn aura_world_swim_7840_is_not_a_run_speed_buff_and_strongest_wins() {
    let mut h = setup(&[7840, 8747], 1);
    for spell in [7840, 8747] {
        apply_native(h.app.world_mut(), h.caster, spell);
    }
    let mods = h.app.world().get::<AuraStatMods>(h.caster).unwrap();
    assert_eq!(mods.swim_speed, 2.0); // +100%, not +115%.
    assert_eq!(mods.movement_speed, 1.0);
    clear(&mut h, 7840);
    assert!(
        (h.app
            .world()
            .get::<AuraStatMods>(h.caster)
            .unwrap()
            .swim_speed
            - 1.15)
            .abs()
            < 0.0001
    );
}

#[test]
fn aura_world_power_display_native_astral_power_restores_mana() {
    let mut h = setup(&[197911], 11);
    apply_native(h.app.world_mut(), h.caster, 197911);
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(h.caster)
            .unwrap()
            .order
            .first(),
        Some(&PowerType::LunarPower)
    );
    clear(&mut h, 197911);
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(h.caster)
            .unwrap()
            .order
            .first(),
        Some(&PowerType::Mana)
    );
}

#[test]
fn aura_world_teleport_origin_snapshots_pose_not_later_movement() {
    let mut h = setup(&[18960], 11);
    let map = crate::maps::InMap::continent(1);
    let position = Position {
        x: 11.0,
        y: 22.0,
        z: 33.0,
    };
    h.app.world_mut().entity_mut(h.caster).insert((
        map,
        position,
        Rotation {
            x: 0.0,
            y: 1.25,
            z: 0.0,
        },
    ));
    apply_native(h.app.world_mut(), h.caster, 18960);
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: 111.0,
        ..position
    });
    let origin = h
        .app
        .world()
        .get::<AuraTeleportOrigins>(h.caster)
        .expect("return locations")
        .0[&18960];
    assert_eq!(origin.map, map);
    assert_eq!(origin.position, position);
    assert_eq!(origin.yaw, 1.25);
    clear(&mut h, 18960);
    assert!(
        !h.app
            .world()
            .get::<AuraTeleportOrigins>(h.caster)
            .unwrap()
            .0
            .contains_key(&18960)
    );
}

/// TC declares these clientside/NULL. Verify authoritative native payload, not invented movement/UI.
#[test]
fn aura_world_client_owned_native_payloads_are_retained() {
    for (id, kind, amount) in [
        (47536, 235, 0.0),
        (101545, 373, 300.0),
        (214202, 188, 20.0),
        (52437, 408, 1.0),
    ] {
        let mut h = setup(&[id], 1);
        apply_native(h.app.world_mut(), h.caster, id);
        let auras = h.app.world().get::<ActiveAuras>(h.caster).unwrap();
        let aura = auras
            .iter()
            .find(|a| a.spell_id == id)
            .expect("native aura retained");
        let effect = aura.effects.iter().find(|e| e.aura_type == kind).unwrap();
        assert_eq!(effect.amount, amount);
        h.advance(1);
        assert!(
            h.app
                .world()
                .get::<UnitAuras>(h.caster)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == id)
        );
    }
}
