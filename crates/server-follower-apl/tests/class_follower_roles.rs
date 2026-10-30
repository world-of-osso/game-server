//! A class follower from the production spawn path casts its spec APL's real spells.
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use server_combat::followers::class_profiles::read_profiles;
use server_combat::followers::class_spawn::{GearSource, spawn_class};
use server_combat::maps::instances::{InstanceEntrant, Now};
use server_combat::spell_cast::test_harness::{
    Harness, ObservedCombatEvents, harness_with_plugins, load_native_creature_world, world_db,
};
use shared::components::{Health, Level};
use shared::protocol::{CombatLogEvent, CombatLogKind};

/// Ten seconds of 20Hz frames: precombat plus several GCDs.
const FRAMES: usize = 200;

struct Pull {
    harness: Harness,
    follower: Entity,
}

fn dungeon_owner(h: &mut Harness) {
    let source = server_combat::test_data::world_data();
    if h.app
        .world()
        .get_resource::<server_combat::world_data::WorldData>()
        .is_none()
    {
        h.app.insert_resource(load_native_creature_world(&[]));
    }
    let mut data = h
        .app
        .world_mut()
        .resource_mut::<server_combat::world_data::WorldData>();
    data.class_level_base_hp = source.class_level_base_hp.clone();
    data.class_level_base_mana = source.class_level_base_mana.clone();
    drop(data);
    let path = server_combat::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let db2 = path.parent().unwrap().join("db2/active");
    h.app
        .insert_resource(server_combat::followers::gear::load_source(&path, &db2).unwrap());
    h.app
        .insert_resource(server_combat::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(server_combat::item_stats::load_item_scaling_tables(&path).unwrap());
    h.app
        .insert_resource(server_combat::stats::NativeItemInstances {
            bonuses: server_combat::item_instance::load::load_bonus_tables(&db).unwrap(),
            enchants: server_combat::item_instance::load::load_enchantment_tables(&db).unwrap(),
            effects: server_combat::item_instance::load::load_item_effect_tables(&db).unwrap(),
        });
    let maps = server_combat::maps::store::load_map_store(&db).unwrap();
    let mut instances = server_combat::maps::InstanceManager::default();
    let entrant = InstanceEntrant {
        character_id: 1,
        leader: 1,
        difficulty_id: 205,
        ..default()
    };
    let now = Now {
        elapsed: 0.0,
        unix: 1_792_454_400,
    };
    let copy = instances.create_map(&maps, 2515, entrant, now).unwrap().map;
    h.app.insert_resource(instances);
    let (owner, dummy) = (h.caster, h.npc);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((copy, Level(90)));
    // A long-lived dummy fighting the owner engages the whole party.
    h.app.world_mut().entity_mut(dummy).insert((
        copy,
        Health {
            current: 10_000_000.0,
            max: 10_000_000.0,
        },
        server_combat::networking::CombatTarget(Some(owner)),
    ));
}

/// Spawn `spec` through `spawn_class` and run the follower brain and its APL for `FRAMES`.
fn pull(spec: u32) -> Pull {
    let mut harness = harness_with_plugins(vec![], &[], 8, false, |app| {
        app.add_plugins(server_spell_scripts::SpellScriptsPlugin);
        app.add_plugins(server_combat::stats::StatsPlugin);
    })
    .expect("native world database");
    let data = world_db().unwrap();
    harness.app.insert_resource(data.store.clone());
    harness.app.insert_resource(data.classes.clone());
    harness.app.init_resource::<ObservedCombatEvents>();
    dungeon_owner(&mut harness);
    let profiles = read_profiles().unwrap();
    let profile = profiles
        .iter()
        .find(|profile| profile.spec_id == spec)
        .unwrap();
    let owner = harness.caster;
    let follower = spawn_class(
        harness.app.world_mut(),
        owner,
        profile,
        1,
        0,
        GearSource::Journal,
    )
    .unwrap();
    // The owner is injured so a healer has real work during the pull.
    let mut health = harness.app.world_mut().get_mut::<Health>(owner).unwrap();
    health.current = health.max * 0.5;
    for _ in 0..FRAMES {
        server_combat::followers::update_ai(harness.app.world_mut());
        server_follower_apl::drive_followers(harness.app.world_mut());
        harness.app.update();
    }
    Pull { harness, follower }
}

impl Pull {
    fn events(&self) -> &[CombatLogEvent] {
        &self
            .harness
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
    }

    /// The APL table's spells as this follower actually casts them (selected overrides included).
    fn resolved(&self, table: impl IntoIterator<Item = u32>) -> Vec<u32> {
        let world = self.harness.app.world();
        table
            .into_iter()
            .flat_map(|spell| {
                let resolved = server_combat::spell_cast::knowledge::resolve_action(
                    world,
                    self.follower,
                    spell,
                )
                .unwrap();
                [Some(spell), resolved].into_iter().flatten()
            })
            .collect()
    }

    fn casts(&self) -> Vec<u32> {
        let follower = Some(self.follower.to_bits());
        self.events()
            .iter()
            .filter(|event| event.source == follower && event.kind == CombatLogKind::CastSuccess)
            .filter_map(|event| event.spell_id)
            .collect()
    }

    /// Whether one of `spells` landed as `kind` on `target`, from this follower.
    fn landed(&self, kind: CombatLogKind, target: Entity, spells: &[u32]) -> bool {
        let follower = Some(self.follower.to_bits());
        self.events().iter().any(|event| {
            event.source == follower
                && event.kind == kind
                && event.target == Some(target.to_bits())
                && event.spell_id.is_some_and(|spell| spells.contains(&spell))
        })
    }
}

#[test]
fn class_follower_tank_casts_protection_apl_spells_on_dummy_pull() {
    let pull = pull(66);
    let table = server_follower_apl::protection_paladin_rotation::context::ACTIONS;
    let apl = pull.resolved(table.iter().map(|&(_, spell)| spell));
    let casts = pull.casts();
    assert!(
        pull.landed(CombatLogKind::Damage, pull.harness.npc, &apl),
        "Protection follower casts {casts:?}; no APL damage {apl:?} on the dummy"
    );
}

#[test]
fn class_follower_healer_casts_holy_priest_apl_heals_on_dummy_pull() {
    let pull = pull(257);
    let heals = server_follower_apl::holy_priest_rotation::SPELLS
        .iter()
        .filter(|(name, _, _)| !matches!(*name, "holy_fire" | "smite"));
    let apl = pull.resolved(heals.map(|&(_, spell, _)| spell));
    let casts = pull.casts();
    assert!(
        pull.landed(CombatLogKind::Heal, pull.harness.caster, &apl),
        "Holy follower casts {casts:?}; no APL heal {apl:?} on the injured owner"
    );
}

#[test]
fn class_follower_dps_casts_arcane_apl_spells_on_dummy_pull() {
    let pull = pull(62);
    let table = server_follower_apl::arcane_rotation::ACTIONS;
    let apl = pull.resolved(table.iter().map(|&(_, spell, _)| spell));
    let casts = pull.casts();
    assert!(
        pull.landed(CombatLogKind::Damage, pull.harness.npc, &apl),
        "Arcane follower casts {casts:?}; no APL damage {apl:?} on the dummy"
    );
}
