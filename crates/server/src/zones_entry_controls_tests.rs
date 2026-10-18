//! Behavioral controls for source UpdateEntry semantics, not full Herald AI acceptance.
use super::entry::AddonPathId;
use super::entry_tests::{source_fixture, spawn_herald};
use super::*;
use crate::scripted_ai::{CreatureAi, Ctx, Hook};

fn update_target(
    data: &WorldData,
    world: &mut World,
    actor: Entity,
    spawn: &CreatureSpawn,
    entry: u32,
    spawn_data: Option<&CreatureSpawn>,
) -> bool {
    let position = *world.get::<Position>(actor).unwrap();
    let map = *world.get::<crate::maps::InMap>(actor).unwrap();
    let placement = CreatureSpawn {
        position_x: position.x,
        position_y: -position.z,
        position_z: position.y,
        ..spawn.clone()
    };
    let Some(projection) = read_entry_projection(
        data,
        entry,
        &mut CreatureSpawnContext {
            map,
            difficulty_id: 1,
            ground: None,
            temporary: false,
        },
        &placement,
        spawn_data,
        &mut rand::thread_rng(),
    ) else {
        return false;
    };
    projection.apply(world.entity_mut(actor), false);
    true
}

#[test]
fn herald_entry_zero_optional_fields_preserve_existing_runtime_state() {
    let (data, spawn) = source_fixture();
    let (mut world, actor) = spawn_herald(&data, &spawn, 1);
    let equipment = data.equipment[&40][&1].clone(); // Actual Kobold Miner1910 pickaxe.
    assert!(!equipment.entries.is_empty());
    let mount = Mounted {
        mount_display_id: 2402,
        ..Default::default()
    };
    let mut pose = *world.get::<UnitPose>(actor).unwrap();
    pose.emote_state = 69; // Prior runtime emote; not an invented Nazan addon row.
    world
        .entity_mut(actor)
        .insert((pose, mount.clone(), equipment.clone(), AddonPathId(0)));
    {
        let mut path = world.get_mut::<WaypointState>(actor).unwrap();
        path.current_index = 2;
        path.delay_remaining = 3.25;
    }
    assert!(update_target(&data, &mut world, actor, &spawn, 17536, None));
    assert_eq!(world.get::<Npc>(actor).unwrap().template_id, 17536);
    assert_eq!(world.get::<Mounted>(actor), Some(&mount));
    assert_eq!(world.get::<EquipmentAppearance>(actor), Some(&equipment));
    assert_eq!(world.get::<UnitPose>(actor).unwrap().emote_state, 69);
    assert_eq!(world.get::<AddonPathId>(actor).unwrap().0, 36215440);
    let path = world.get::<WaypointState>(actor).unwrap();
    assert_eq!((path.current_index, path.delay_remaining), (2, 3.25));
    assert_eq!(
        path.path_id, 36215440,
        "entry update does not restart the generator"
    );
}

#[test]
fn herald_entry_explicit_equipment_zero_clears_without_replacing_actor() {
    let (data, spawn) = source_fixture();
    let (mut world, actor) = spawn_herald(&data, &spawn, 1);
    world
        .entity_mut(actor)
        .insert(data.equipment[&40][&1].clone());
    assert_eq!(spawn.equipment_id, 0);
    assert!(update_target(
        &data,
        &mut world,
        actor,
        &spawn,
        17536,
        Some(&spawn)
    ));
    assert!(world.get::<EquipmentAppearance>(actor).is_none());
    assert_eq!(world.get::<Npc>(actor).unwrap().template_id, 17536);
    assert_eq!(world.query::<&Npc>().iter(&world).count(), 1);
}

#[test]
fn herald_entry_missing_template_refuses_before_actor_mutation() {
    let (data, spawn) = source_fixture();
    let (mut world, actor) = spawn_herald(&data, &spawn, 1);
    let old_health = *world.get::<Health>(actor).unwrap();
    assert!(!update_target(
        &data,
        &mut world,
        actor,
        &spawn,
        u32::MAX,
        None
    ));
    assert_eq!(world.get::<Npc>(actor).unwrap().template_id, 17307);
    assert_eq!(*world.get::<Health>(actor).unwrap(), old_health);
}

#[derive(Component, Default)]
struct Observations {
    fired: u32,
    resets: u32,
    appeared: u32,
}

struct TimerProbe {
    events: crate::scripted_ai::event_map::EventMap,
}

impl CreatureAi for TimerProbe {
    fn reset(&mut self, ctx: &mut Ctx) {
        ctx.world.get_mut::<Observations>(ctx.me).unwrap().resets += 1;
    }
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.world.get_mut::<Observations>(ctx.me).unwrap().appeared += 1;
    }
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.events.update(u64::from(diff_ms));
        while self.events.execute_event().is_some() {
            ctx.world.get_mut::<Observations>(ctx.me).unwrap().fired += 1;
        }
    }
}

#[test]
fn herald_entry_preserves_real_ai_event_progress_without_appearance_replay() {
    let (data, spawn) = source_fixture();
    let (mut world, actor) = spawn_herald(&data, &spawn, 1);
    world.entity_mut(actor).insert(Observations::default());
    let mut events = crate::scripted_ai::event_map::EventMap::default();
    events.schedule(7, 100);
    crate::scripted_ai::tests::install_fixture_ai(
        &mut world,
        actor,
        Box::new(TimerProbe { events }),
    );
    crate::scripted_ai::dispatch(&mut world, actor, Hook::UpdateAi(50));
    assert_eq!(world.get::<Observations>(actor).unwrap().fired, 0);
    assert!(update_target(&data, &mut world, actor, &spawn, 17536, None));
    assert_eq!(world.get::<Npc>(actor).unwrap().template_id, 17536);
    crate::scripted_ai::dispatch(&mut world, actor, Hook::UpdateAi(49));
    assert_eq!(world.get::<Observations>(actor).unwrap().fired, 0);
    crate::scripted_ai::dispatch(&mut world, actor, Hook::UpdateAi(1));
    let observed = world.get::<Observations>(actor).unwrap();
    assert_eq!(
        (observed.fired, observed.resets, observed.appeared),
        (1, 0, 0)
    );
}

#[test]
fn herald_entry_real_addon_auras_are_additive_and_existing_instances_are_not_refreshed() {
    let (mut data, _) = source_fixture();
    let canonical = crate::test_data::world_data();
    for entry in [30, 100] {
        data.templates
            .insert(entry, canonical.templates[&entry].clone());
    }
    let spawn = canonical
        .spawns
        .iter()
        .find(|spawn| {
            spawn.template_entry == 30
                && spawn.map == 0
                && !canonical.addons.contains_key(&spawn.guid)
        })
        .expect("real Forest Spider without a per-spawn addon")
        .clone();
    assert_eq!(data.template_addons[&30].auras, vec![11959]);
    assert_eq!(data.template_addons[&100].auras, vec![3417]);
    let mut world = World::new();
    world.insert_resource(Time::<bevy::time::Virtual>::default());
    world.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let actor = spawn_creature(
        &mut Commands::new(&mut queue, &world),
        &data,
        &data.templates[&30],
        &mut CreatureSpawnContext {
            map: crate::maps::InMap::continent(0),
            difficulty_id: 0,
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        &spawn,
        &mut rand::thread_rng(),
    )
    .unwrap();
    queue.apply(&mut world);
    apply_addon_auras(&mut world);
    let previous = world
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 11959)
        .unwrap()
        .clone();
    assert!(update_target(&data, &mut world, actor, &spawn, 100, None));
    assert!(
        crate::auras::has_aura(&world, actor, 3417),
        "new source addon aura is applied now"
    );
    assert_eq!(
        world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 11959),
        Some(&previous)
    );
    let added = world
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 3417)
        .unwrap()
        .clone();
    world
        .resource_mut::<Time<bevy::time::Virtual>>()
        .advance_by(std::time::Duration::from_secs(1));
    assert!(update_target(&data, &mut world, actor, &spawn, 100, None));
    assert_eq!(
        world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 3417),
        Some(&added)
    );
}
