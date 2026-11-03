//! A class follower from the production spawn path casts its spec APL's real spells.
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use server_combat::followers::class_profiles::read_profiles;
use server_combat::followers::class_spawn::{GearSource, spawn_class};
use server_combat::maps::instances::{InstanceEntrant, Now};
use server_combat::spell_cast::test_harness::{
    Harness, ObservedCombatEvents, harness_with_plugins, world_db,
};
use server_follower_apl::{FollowerApl, FollowerAplFailed};
use shared::components::{Health, Level};
use shared::protocol::{CombatLogEvent, CombatLogKind};

/// Ten seconds of 20Hz frames: precombat plus several GCDs.
const FRAMES: usize = 200;
const STOCKADE: u32 = 34;

struct Pull {
    harness: Harness,
    follower: Entity,
}

fn dungeon_owner(h: &mut Harness, map: u32) {
    // Stockade is the Normal-only simulator map; follower dungeons run difficulty 205.
    let difficulty = if map == STOCKADE { 1 } else { 205 };
    // Production world data, as the server loads it at startup, plus any
    // creature templates this harness authored for its own units.
    let source = server_combat::test_data::world_data();
    let mut data = source.clone();
    if let Some(harness) = h
        .app
        .world()
        .get_resource::<server_combat::world_data::WorldData>()
    {
        for (entry, template) in harness.templates.iter() {
            data.templates
                .entry(*entry)
                .or_insert_with(|| template.clone());
        }
    }
    h.app.insert_resource(data);
    // Production world startup inserts the armor constants Stagger and mitigation read.
    h.app.insert_resource(source.armor_constants.clone());
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
    h.app
        .insert_resource(server_combat::item_proc_data::load(&db).unwrap());
    h.app
        .insert_resource(server_combat::hunter_pet::read_pet_catalog(&db).unwrap());
    h.app
        .insert_resource(server_combat::followers::load_catalog(&db).unwrap());
    let maps = server_combat::maps::store::load_map_store(&db).unwrap();
    let mut instances = server_combat::maps::InstanceManager::default();
    let entrant = InstanceEntrant {
        character_id: 1,
        leader: 1,
        difficulty_id: difficulty,
        ..default()
    };
    let now = Now {
        elapsed: 0.0,
        unix: 1_792_454_400,
    };
    let copy = instances.create_map(&maps, map, entrant, now).unwrap().map;
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

#[test]
fn rotation_interrupts_unselected_engaged_channel_without_switching_damage_target() {
    use server_combat::networking::CombatTarget;
    use shared::casting::CastState;
    let mut pull = spawn(581);
    let actor = pull.follower;
    let primary = pull.harness.npc;
    let map = *pull
        .harness
        .app
        .world()
        .get::<server_combat::maps::InMap>(primary)
        .unwrap();
    let position = *pull
        .harness
        .app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap();
    let other = server_combat::spell_cast::test_harness::spawn_npc(&mut pull.harness.app, 0.0);
    pull.harness.app.world_mut().entity_mut(other).insert((
        map,
        position,
        CombatTarget(Some(actor)),
    ));
    pull.harness
        .app
        .world_mut()
        .entity_mut(actor)
        .insert(CombatTarget(Some(primary)));
    let mut channel = CastState::channel(1264106, other.to_bits(), 180.0, 1.0, true);
    channel.elapsed = 2.0;
    pull.harness
        .app
        .world_mut()
        .entity_mut(other)
        .insert(channel);
    pull.harness.app.update();
    assert!(
        pull.harness.app.world().get::<CastState>(other).is_some(),
        "channel prerequisite"
    );
    pull.harness
        .app
        .world_mut()
        .entity_mut(actor)
        .insert(CombatTarget(Some(primary)));
    let rotation =
        server_follower_apl::registry::load(pull.harness.app.world_mut(), actor, 581).unwrap();
    let mut apl = FollowerApl::new(rotation);
    apl.tick(pull.harness.app.world_mut(), actor, &[primary, other])
        .unwrap();
    assert!(
        pull.harness.app.world().get::<CastState>(other).is_none(),
        "unselected channel remains active"
    );
    assert_eq!(
        pull.harness
            .app
            .world()
            .get::<CombatTarget>(actor)
            .unwrap()
            .0,
        Some(primary)
    );
}

/// Spawn `spec` through `spawn_class` and run the follower brain and its APL for `FRAMES`.
fn pull(spec: u32) -> Pull {
    let mut pull = spawn(spec);
    pull.fight();
    pull
}

fn spawn(spec: u32) -> Pull {
    spawn_at(spec, 90)
}

#[test]
fn fresh_vengeance_follower_has_out_of_combat_status_on_first_apl_tick() {
    use server_combat::simc_apl::Facts;
    use server_follower_apl::vengeance_tank_rotation::{Context, Rotation};
    use shared::components::CombatStatus;

    let mut pull = spawn(581);
    pull.harness
        .app
        .insert_resource(server_combat::test_support::GroundMap::flat_arena(0.0));
    let actor = pull.follower;
    let target = pull.harness.npc;
    let world = pull.harness.app.world_mut();
    assert_eq!(world.get::<CombatStatus>(actor), Some(&CombatStatus(false)));
    let encounter = server_combat::simc_apl_native::Encounter::dungeon(0.0);
    let rotation = Rotation::load(world, actor, encounter).unwrap();
    let targets = [target];
    let facts = Context::new(
        world,
        actor,
        &targets,
        &rotation.bindings,
        &rotation.equipment,
        encounter,
    );
    assert_eq!(facts.value("in_combat").unwrap(), 0.0);
    let mut apl = FollowerApl::new_for_world(world, Box::new(rotation));
    apl.tick(world, actor, &targets)
        .expect("first Vengeance APL tick");
}

/// Spawn `spec` for an owner at `level`; followers take their owner's level.
fn spawn_at(spec: u32, level: u8) -> Pull {
    spawn_at_map(spec, level, 2515)
}

fn spawn_at_map(spec: u32, level: u8, map: u32) -> Pull {
    let mut harness = harness_with_plugins(vec![], &[], 8, false, |app| {
        app.add_plugins(server_spell_scripts::SpellScriptsPlugin);
        app.add_plugins(server_combat::stats::StatsPlugin);
        server_follower_apl::action_metadata::install_provider(app);
    })
    .expect("native world database");
    let data = world_db().unwrap();
    harness.app.insert_resource(data.spell_store().clone());
    harness.app.insert_resource(data.class_spells().clone());
    harness.app.init_resource::<ObservedCombatEvents>();
    dungeon_owner(&mut harness, map);
    let owner = harness.caster;
    harness
        .app
        .world_mut()
        .entity_mut(owner)
        .insert(Level(level));
    let profiles = read_profiles().unwrap();
    let profile = profiles
        .iter()
        .find(|profile| profile.spec_id == spec)
        .unwrap();
    let follower = spawn_class(
        harness.app.world_mut(),
        owner,
        profile,
        1,
        0,
        GearSource::Journal,
    )
    .unwrap();
    // The owner, with a level-90 pool like the follower's, is half injured
    // so a healer has real work during the pull.
    let max = harness.app.world().get::<Health>(follower).unwrap().max;
    harness.app.world_mut().entity_mut(owner).insert(Health {
        current: max * 0.5,
        max,
    });
    Pull { harness, follower }
}

impl Pull {
    /// Run the follower brain and its APL for `FRAMES`.
    fn fight(&mut self) {
        for _ in 0..FRAMES {
            server_combat::followers::update_ai(self.harness.app.world_mut());
            server_follower_apl::drive_followers(self.harness.app.world_mut());
            self.harness.app.update();
        }
    }

    fn apl_failed(&self) -> bool {
        self.harness
            .app
            .world()
            .get::<FollowerAplFailed>(self.follower)
            .is_some()
    }

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
fn follower_healer_casts_on_nearby_heal_objective() {
    use shared::components::{Npc, Position, UnitFactionTemplate};
    let mut pull = spawn_at_map(257, 90, 2515);
    let actor = pull.follower;
    let owner = pull.harness.caster;
    let world = pull.harness.app.world_mut();
    let map = server_combat::maps::InMap {
        map_id: 1877,
        instance_id: 1,
    };
    world.entity_mut(actor).insert(map);
    world.entity_mut(owner).insert(map);
    let position = *world.get::<Position>(actor).unwrap();
    // TDB 69497 Avatar faction and unit flags from the population candidate.
    let faction = UnitFactionTemplate(1665);
    let max = world.get::<Health>(actor).unwrap().max;
    world.get_mut::<Health>(owner).unwrap().current = max;
    let path = server_combat::test_data::world_db_path();
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut maps = server_combat::maps::store::load_map_store(&db).unwrap();
    maps.encounter_heal_objectives.insert((1877, 133392), 25.0);
    world.insert_resource(maps);
    assert!(server_combat::scripted_ai::instance_script::set_boss_state(
        world,
        map,
        3,
        server_combat::scripted_ai::instance_script::EncounterState::InProgress
    ));
    let avatar = world
        .spawn((
            map,
            position,
            faction,
            shared::components::UnitFlags(832),
            Npc {
                template_id: 133392,
                name: "Avatar of Sethraliss".into(),
            },
            Health {
                current: max * 0.25,
                max,
            },
        ))
        .id();
    assert!(server_follower_apl::healing_party(world, actor).contains(&avatar));
    world.get_mut::<Position>(avatar).unwrap().x += 100.0;
    assert!(
        !server_follower_apl::healing_party(world, actor).contains(&avatar),
        "objectives outside the native spell range are not selected"
    );
    world.entity_mut(avatar).insert((
        position,
        server_combat::maps::InMap {
            instance_id: map.instance_id + 1,
            ..map
        },
    ));
    assert!(
        !server_follower_apl::healing_party(world, actor).contains(&avatar),
        "another instance copy is not selected"
    );
    world.entity_mut(avatar).insert(map);
    server_combat::auras::apply_aura(
        world,
        &server_combat::auras::AuraApplication {
            caster: avatar,
            target: avatar,
            spell_id: 1301202,
            effect_mask: 0b1110,
            source: server_combat::auras::AuraSource::Cast,
        },
    );
    assert!(
        !server_follower_apl::healing_party(world, actor).contains(&avatar),
        "a healer must not select the Taint-protected Avatar"
    );
    server_combat::auras::remove_auras(
        world,
        avatar,
        server_combat::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1301202,
    );
    assert!(server_follower_apl::healing_party(world, actor).contains(&avatar));
    pull.fight();
    assert!(
        pull.harness
            .app
            .world()
            .get::<Health>(avatar)
            .unwrap()
            .current
            > max * 0.25,
        "follower must land an actual heal on the objective"
    );
}

#[test]
fn follower_weapon_action_fury_satisfies_both_levels() {
    for (level, map) in [(70, 2515), (90, 2813)] {
        let mut pull = spawn_at_map(72, level, map);
        let actor = pull.follower;
        let world = pull.harness.app.world_mut();
        let rotation = server_follower_apl::registry::load(world, actor, 72).unwrap();
        let actions = rotation.action_spells();
        for spell in [85288, 184367, 335097] {
            assert!(
                actions.contains(&spell),
                "level {level} missing bound action {spell}"
            );
            let info = world
                .resource::<server_combat::spell_info::SpellStore>()
                .get(spell)
                .unwrap();
            server_combat::spell_cast::check_equipped_item_requirements(world, actor, info)
                .unwrap_or_else(|error| panic!("level {level}, spell {spell}: {error:?}"));
        }
    }
}

#[test]
fn follower_weapon_action_elemental_equips_shield() {
    let mut pull = spawn_at_map(262, 90, 2813);
    let actor = pull.follower;
    let world = pull.harness.app.world_mut();
    let rotation = server_follower_apl::registry::load(world, actor, 262).unwrap();
    assert!(rotation.action_spells().contains(&462757));
    let info = world
        .resource::<server_combat::spell_info::SpellStore>()
        .get(462757)
        .unwrap();
    server_combat::spell_cast::check_equipped_item_requirements(world, actor, info).unwrap();
}

#[test]
fn follower_weapon_action_protection_equips_tier_shield_in_stockade() {
    let mut pull = spawn_at_map(66, 32, STOCKADE);
    let actor = pull.follower;
    let world = pull.harness.app.world_mut();
    let (_, source, item) = *world
        .get::<server_combat::stats::EquippedItems>(actor)
        .unwrap()
        .items
        .iter()
        .find(|(slot, _, _)| *slot == shared::protocol::EquipmentSlot::OffHand)
        .expect("Stockade Protection follower equips an off-hand");
    let template = world
        .resource::<server_combat::item_templates::ItemTemplates>()
        .get(source, item)
        .unwrap();
    assert_eq!(
        template.inventory_type, 14,
        "off-hand item{item} is a shield"
    );
    let info = world
        .resource::<server_combat::spell_info::SpellStore>()
        .get(31935)
        .unwrap();
    server_combat::spell_cast::check_equipped_item_requirements(world, actor, info).unwrap();
}

#[test]
#[ignore = "audits all class-follower action requirements against snapshot gear"]
fn class_follower_gear_satisfies_all_bound_action_requirements() {
    let mut failures = Vec::new();
    let profiles = read_profiles().unwrap();
    for (level, map) in [(70, 2515), (90, 2813)] {
        for profile in &profiles {
            let mut pull = spawn_at_map(profile.spec_id, level, map);
            let actor = pull.follower;
            let world = pull.harness.app.world_mut();
            let rotation = match server_follower_apl::registry::load(world, actor, profile.spec_id)
            {
                Ok(rotation) => rotation,
                Err(error) => {
                    failures.push((level, profile.spec_id, 0, error));
                    continue;
                }
            };
            for spell in rotation.action_spells() {
                let Some(info) = world
                    .resource::<server_combat::spell_info::SpellStore>()
                    .get(spell)
                else {
                    failures.push((
                        level,
                        profile.spec_id,
                        spell,
                        "bound spell metadata absent".to_owned(),
                    ));
                    continue;
                };
                if let Err(error) =
                    server_combat::spell_cast::check_equipped_item_requirements(world, actor, info)
                {
                    failures.push((level, profile.spec_id, spell, format!("{error:?}")));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "bound action equipment refusals: {failures:?}"
    );
}

#[test]
fn pure_action_metadata_matches_every_class_rotation() {
    for &spec in server_follower_apl::registry::SPECS {
        let mut pull = spawn(spec);
        let rotation =
            server_follower_apl::registry::load(pull.harness.app.world_mut(), pull.follower, spec)
                .unwrap();
        let input = server_follower_apl::action_metadata::read_inputs(
            pull.harness.app.world(),
            pull.follower,
            spec,
        )
        .unwrap();
        let pure = server_follower_apl::action_metadata::action_spells(spec, &input).unwrap();
        let (program, mut bindings) =
            server_follower_apl::action_metadata::parse(spec, &input).unwrap();
        let mut expected = pure.clone();
        expected.extend(
            server_follower_apl::action_metadata::equipped_action_spells(
                pull.harness.app.world(),
                pull.follower,
                &program,
                &mut bindings,
            )
            .unwrap(),
        );
        expected.sort_unstable();
        expected.dedup();
        assert_eq!(
            pull.harness
                .app
                .world()
                .get::<server_combat::followers::FollowerActionSpells>(pull.follower)
                .unwrap()
                .0,
            expected,
            "spec{spec} spawn metadata differs"
        );
        assert!(!pure.is_empty(), "spec{spec} has no bound actions");
        assert_eq!(
            pure,
            rotation.action_spells(),
            "spec{spec} pure and runtime actions differ"
        );
        assert!(pure.windows(2).all(|pair| pair[0] < pair[1]));
        if let Some(spell) = [(63, 133), (257, 2061), (66, 31935)]
            .into_iter()
            .find_map(|(id, spell)| (id == spec).then_some(spell))
        {
            assert!(
                pure.contains(&spell),
                "spec{spec} missing {spell}: {pure:?}"
            );
        }
    }
}

#[test]
fn pure_metadata_includes_equipped_hostile_area_item_only_when_program_uses_items() {
    let mut pull = spawn(63);
    let world = pull.harness.app.world_mut();
    let mut items = world
        .get_mut::<server_combat::stats::EquippedItems>(pull.follower)
        .unwrap();
    items
        .items
        .retain(|(slot, _, _)| *slot != shared::protocol::EquipmentSlot::Trinket1);
    items.items.push((
        shared::protocol::EquipmentSlot::Trinket1,
        shared::item_data::ItemDefinitionSource::Retail,
        204736,
    ));
    let provider = *world.resource::<server_combat::followers::FollowerActionProvider>();
    let mut action_spells = (provider.source_actions)(world, pull.follower, 63).unwrap();
    action_spells.extend((provider.equipped_actions)(world, pull.follower, 63).unwrap());
    action_spells.sort_unstable();
    action_spells.dedup();
    world
        .entity_mut(pull.follower)
        .insert(server_combat::followers::FollowerActionSpells(
            action_spells,
        ));
    assert!(
        world
            .get::<server_combat::followers::FollowerActionSpells>(pull.follower)
            .unwrap()
            .0
            .contains(&407512)
    );
    let input =
        server_follower_apl::action_metadata::read_inputs(world, pull.follower, 63).unwrap();
    let (program, mut bindings) = server_follower_apl::action_metadata::parse(63, &input).unwrap();
    let spells = server_follower_apl::action_metadata::equipped_action_spells(
        world,
        pull.follower,
        &program,
        &mut bindings,
    )
    .unwrap();
    assert!(
        spells.contains(&407512),
        "equipped hostile item spell missing: {spells:?}"
    );
    let reach = server_combat::followers::spell_unintended_reach(
        world,
        pull.follower,
        407512,
        pull.follower,
    )
    .unwrap();
    assert!(
        reach.0 > 0.0 || reach.1 > 0.0,
        "item must have real hostile area reach: {reach:?}"
    );
    world
        .entity_mut(pull.follower)
        .insert(server_combat::followers::FollowerActionSpells(vec![407512]));
    let aggregate = server_combat::followers::party_unintended_reach(world, &[pull.follower]);
    assert!(
        aggregate
            .iter()
            .any(|&(actor, caster, target)| actor == pull.follower
                && caster >= reach.0
                && target >= reach.1),
        "equipped item reach missing: {aggregate:?}"
    );
    let no_items = server_combat::simc_apl::Program::parse(&[("default", "fireball")]).unwrap();
    let absent = server_follower_apl::action_metadata::equipped_action_spells(
        world,
        pull.follower,
        &no_items,
        &mut bindings,
    )
    .unwrap();
    assert!(
        absent.is_empty(),
        "item not named by program must not enter metadata: {absent:?}"
    );
}

#[test]
fn fire_follower_reach_uses_program_actions_without_the_kit_catalog() {
    let mut pull = spawn(63);
    let actor = pull.follower;
    let world = pull.harness.app.world_mut();
    let actions = &world
        .get::<server_combat::followers::FollowerActionSpells>(actor)
        .unwrap()
        .0;
    assert!(
        actions.contains(&108853),
        "Fire Blast must be a selected APL action"
    );
    let blast =
        server_combat::followers::spell_unintended_reach(world, actor, 108853, actor).unwrap();
    assert!(
        blast.0 > 0.0,
        "native Fire Blast spread reach must be visible"
    );
    world.remove_resource::<server_combat::followers::FollowerCatalog>();
    let reaches = server_combat::followers::party_unintended_reach(world, &[actor]);
    assert!(
        reaches
            .iter()
            .any(|&(unit, caster, _)| unit == actor && caster >= blast.0)
    );
}

#[test]
fn class_healer_pull_readiness_uses_program_heals_without_the_kit_catalog() {
    let mut pull = spawn(257);
    let healer = pull.follower;
    let tank = pull.harness.caster;
    let world = pull.harness.app.world_mut();
    let actions = world
        .get::<server_combat::followers::FollowerActionSpells>(healer)
        .unwrap()
        .0
        .clone();
    assert!(
        actions.contains(&2061),
        "Holy Flash Heal must be a selected APL action"
    );
    world.remove_resource::<server_combat::followers::FollowerCatalog>();
    let (ready_healer, spell) = server_combat::followers::test_ready_heal(world, tank, &[healer])
        .unwrap()
        .expect("a sourced APL heal must be ready");
    assert_eq!(ready_healer, healer);
    assert!(
        actions.contains(&spell),
        "ready heal {spell} not in APL actions {actions:?}"
    );
    world
        .entity_mut(healer)
        .insert(server_combat::followers::FollowerActionSpells(Vec::new()));
    assert!(
        server_combat::followers::test_ready_heal(world, tank, &[healer])
            .unwrap()
            .is_none()
    );
}

#[test]
fn every_class_tank_taunts_the_healers_attacker_with_its_known_spell() {
    let mut failures = Vec::new();
    for (spec, taunt) in [
        (66, 62124),
        (73, 355),
        (104, 6795),
        (250, 56222),
        (268, 115546),
        (581, 185245),
    ] {
        let mut pull = spawn(spec);
        let owner = pull.harness.caster;
        let enemy = pull.harness.npc;
        let healer_profile = read_profiles()
            .unwrap()
            .into_iter()
            .find(|profile| profile.spec_id == 257)
            .unwrap();
        let world = pull.harness.app.world_mut();
        let healer = spawn_class(world, owner, &healer_profile, 2, 0, GearSource::Journal).unwrap();
        world
            .get_mut::<server_combat::followers::Follower>(pull.follower)
            .unwrap()
            .profile
            .taunt_spell = 133; // The live class taunt must not come from this legacy field.
        world.entity_mut(enemy).insert((
            shared::components::ThreatTable::default(),
            server_combat::networking::CombatTarget(Some(healer)),
            server_combat::creature_ai::CreatureState::Pursuing {
                target: healer.to_bits(),
            },
        ));
        server_combat::creature_aggro::add_threat(world, enemy, healer, 100.0);
        for _ in 0..120 {
            server_combat::followers::update_ai(pull.harness.app.world_mut());
            server_follower_apl::drive_followers(pull.harness.app.world_mut());
            pull.harness.app.update();
            if pull.casts().contains(&taunt) {
                break;
            }
        }
        if !pull.casts().contains(&taunt) {
            let world = pull.harness.app.world_mut();
            let target = world
                .get::<server_combat::networking::CombatTarget>(pull.follower)
                .and_then(|target| target.0);
            let victim = world
                .get::<server_combat::networking::CombatTarget>(enemy)
                .and_then(|target| target.0);
            let known = server_combat::spell_cast::knowledge::resolve_actor_action(
                world,
                pull.follower,
                taunt,
            );
            let candidates: Vec<_> = world
                .get::<server_combat::class_progression::KnownSpells>(pull.follower)
                .unwrap()
                .iter()
                .filter(|&id| {
                    world
                        .resource::<server_combat::spell_info::SpellStore>()
                        .get(id)
                        .is_some_and(|spell| {
                            spell.effects.iter().any(|effect| {
                                effect.effect
                                    == server_combat::spell_cast::effects::EFFECT_ATTACK_ME
                                    || effect.aura == server_combat::auras::handlers::MOD_TAUNT
                            })
                        })
                })
                .collect();
            let spell = world
                .resource::<server_combat::spell_info::SpellStore>()
                .get(taunt)
                .unwrap()
                .clone();
            let admission = server_combat::spell_cast::read_native_cast_start(
                world,
                pull.follower,
                &spell,
                enemy,
            );
            failures.push(format!(
                "spec {spec} did not cast {taunt}: target={target:?} victim={victim:?} known={known:?} candidates={candidates:?} admission={admission:?}"
            ));
        } else if pull
            .harness
            .app
            .world()
            .get::<server_combat::networking::CombatTarget>(enemy)
            .unwrap()
            .0
            != Some(pull.follower)
        {
            failures.push(format!(
                "spec {spec} did not take the enemy from the healer"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn class_follower_holds_damage_during_polymorph_and_resumes_afterward() {
    let mut pull = spawn(63);
    let owner = pull.harness.caster;
    let target = pull.harness.npc;
    let world = pull.harness.app.world_mut();
    server_combat::creature_aggro::add_threat(world, target, owner, 100.0);
    let spell = world
        .resource::<server_combat::spell_info::SpellStore>()
        .get(118)
        .unwrap();
    let mask = server_combat::auras::aura_effect_mask(spell);
    server_combat::auras::apply_aura(
        world,
        &server_combat::auras::AuraApplication {
            caster: owner,
            target,
            spell_id: 118,
            effect_mask: mask,
            source: server_combat::auras::AuraSource::Cast,
        },
    );
    for _ in 0..40 {
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
    }
    assert!(server_combat::auras::has_aura(
        pull.harness.app.world(),
        target,
        118
    ));
    assert!(
        !pull
            .events()
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.source == Some(pull.follower.to_bits())
                && event.target == Some(target.to_bits()))
    );
    server_combat::auras::remove_auras(
        pull.harness.app.world_mut(),
        target,
        server_combat::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 118,
    );
    for _ in 0..200 {
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
        if pull.events().iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(pull.follower.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0
        }) {
            return;
        }
    }
    panic!(
        "class follower did not resume damage after polymorph ended: {:?}",
        pull.casts()
    );
}

#[test]
fn class_tank_defends_against_an_engaged_alternate_after_its_primary_dies() {
    let mut pull = spawn(66);
    let owner = pull.harness.caster;
    let primary = pull.harness.npc;
    let alternate = server_combat::spell_cast::test_harness::spawn_npc(&mut pull.harness.app, 3.0);
    let world = pull.harness.app.world_mut();
    let map = *world.get::<server_combat::maps::InMap>(primary).unwrap();
    world.entity_mut(alternate).insert((
        map,
        Health {
            current: 10_000_000.0,
            max: 10_000_000.0,
        },
        shared::components::ThreatTable::default(),
    ));
    server_combat::creature_aggro::add_threat(world, primary, owner, 100.0);
    server_combat::creature_aggro::add_threat(world, alternate, owner, 100.0);
    server_combat::combat::kill_unit(world, owner, primary);
    for _ in 0..200 {
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
        if pull.events().iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(pull.follower.to_bits())
                && event.target == Some(alternate.to_bits())
                && event.amount > 0
        }) {
            return;
        }
    }
    panic!(
        "class tank did not damage the engaged alternate after primary death: {:?}",
        pull.casts()
    );
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

/// Holy Armaments alternates Holy Bulwark and Sacred Weapon through its override
/// aura; the Protection APL must keep running across both armaments.
#[test]
fn class_follower_protection_apl_keeps_casting_through_both_holy_armaments() {
    let mut pull = spawn(66);
    let (follower, dummy) = (pull.follower, pull.harness.npc);
    let rotation =
        server_follower_apl::registry::load(pull.harness.app.world_mut(), follower, 66).unwrap();
    let mut apl = FollowerApl::new(rotation);
    // One minute: Holy Armaments (2 charges) is cast and its armament flips.
    for frame in 0..1200 {
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        if let Err(error) = apl.tick(pull.harness.app.world_mut(), follower, &[dummy]) {
            panic!("frame {frame}: {error}; casts {:?}", pull.casts());
        }
        pull.harness.app.update();
    }
    let casts = pull.casts();
    for armament in [432459, 432472] {
        assert!(
            casts.contains(&armament),
            "{armament} not in casts {casts:?}"
        );
    }
}

/// Below hero-talent level Holy Armaments is unknown; SimC skips the action and
/// the rest of the Protection APL keeps running.
#[test]
fn class_follower_protection_apl_casts_without_hero_talents() {
    let mut pull = spawn_at(66, 70);
    let world = pull.harness.app.world();
    let armaments =
        server_combat::spell_cast::knowledge::resolve_action(world, pull.follower, 432459);
    assert_eq!(
        armaments,
        Ok(None),
        "a level-70 Protection follower knows Holy Armaments"
    );
    pull.fight();
    assert!(
        !pull.apl_failed(),
        "Protection APL stopped; casts {:?}",
        pull.casts()
    );
    let table = server_follower_apl::protection_paladin_rotation::context::ACTIONS;
    let apl = pull.resolved(table.iter().map(|&(_, spell)| spell));
    assert!(
        pull.landed(CombatLogKind::Damage, pull.harness.npc, &apl),
        "casts {:?}",
        pull.casts()
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
fn class_healer_heals_the_most_injured_party_member_before_damage() {
    let mut pull = spawn(257);
    let owner = pull.harness.caster;
    let profiles = read_profiles().unwrap();
    let arcane = profiles
        .iter()
        .find(|profile| profile.spec_id == 62)
        .unwrap();
    let ally = spawn_class(
        pull.harness.app.world_mut(),
        owner,
        arcane,
        1,
        1,
        GearSource::Journal,
    )
    .unwrap();
    // The owner is at half health; the attacking ally is far lower.
    let max = pull.harness.app.world().get::<Health>(ally).unwrap().max;
    pull.harness
        .app
        .world_mut()
        .entity_mut(ally)
        .insert(Health {
            current: max * 0.1,
            max,
        });
    let mana = pull
        .harness
        .app
        .world()
        .get::<shared::components::Mana>(pull.follower)
        .unwrap()
        .current;
    // Halo heals and damages in one area cast; the APL's heal priority is its
    // targeted heals and its separate damage list.
    let table = |names: &[&str]| {
        pull.resolved(
            server_follower_apl::holy_priest_rotation::SPELLS
                .iter()
                .filter(|(name, _, _)| names.contains(name))
                .map(|&(_, spell, _)| spell),
        )
    };
    let targeted_heals = table(&[
        "guardian_spirit",
        "serenity",
        "prayer_of_mending",
        "renew",
        "flash_heal",
        "heal",
    ]);
    let damage = table(&["holy_fire", "smite"]);
    let healer = Some(pull.follower.to_bits());
    let first_heal = |pull: &Pull| {
        pull.events().iter().position(|event| {
            event.source == healer
                && event.kind == CombatLogKind::CastSuccess
                && event
                    .spell_id
                    .is_some_and(|spell| targeted_heals.contains(&spell))
        })
    };
    for _ in 0..FRAMES {
        if first_heal(&pull).is_some() {
            break;
        }
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
    }
    let first_heal = first_heal(&pull).expect("Holy follower must cast a targeted APL heal");
    assert_eq!(
        pull.events()[first_heal].target,
        Some(ally.to_bits()),
        "Holy follower must heal the most injured ally first"
    );
    let early_damage: Vec<_> = pull.events()[..first_heal]
        .iter()
        .filter(|event| {
            event.source == healer
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id.is_some_and(|spell| damage.contains(&spell))
        })
        .collect();
    assert!(
        early_damage.is_empty(),
        "Holy damage list ran while a heal was due: {early_damage:?}"
    );
    assert!(
        pull.harness
            .app
            .world()
            .get::<shared::components::Mana>(pull.follower)
            .unwrap()
            .current
            < mana,
        "ordinary heals must spend mana"
    );
}

#[test]
fn class_healer_uses_apl_heals_without_damage_during_pull_back() {
    let mut pull = spawn(257);
    let owner = pull.harness.caster;
    let profiles = read_profiles().unwrap();
    let tank = profiles
        .iter()
        .find(|profile| profile.spec_id == 66)
        .unwrap();
    let tank = spawn_class(
        pull.harness.app.world_mut(),
        owner,
        tank,
        1,
        1,
        GearSource::Journal,
    )
    .unwrap();
    // Hold the tank's pull-back state fixed while testing the healer's controller.
    pull.harness
        .app
        .world_mut()
        .entity_mut(tank)
        .remove::<server_combat::followers::Follower>();
    server_combat::followers::set_test_pull_back(pull.harness.app.world_mut(), owner, tank);
    assert!(server_combat::followers::holding_pull_damage(
        pull.harness.app.world(),
        pull.follower
    ));
    let before = pull.casts().len();
    for _ in 0..5 {
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        pull.harness.app.update();
    }
    assert_eq!(
        pull.casts().len(),
        before,
        "follower brain cast a kit action"
    );
    assert!(
        pull.harness
            .app
            .world()
            .get::<shared::casting::CastState>(pull.follower)
            .is_none(),
        "follower brain started a kit cast"
    );
    for _ in 0..FRAMES {
        assert!(server_combat::followers::holding_pull_damage(
            pull.harness.app.world(),
            pull.follower
        ));
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
    }
    let heals = server_follower_apl::holy_priest_rotation::SPELLS
        .iter()
        .filter(|(name, _, _)| !matches!(*name, "holy_fire" | "smite"));
    let apl = pull.resolved(heals.map(|&(_, spell, _)| spell));
    assert!(
        pull.landed(CombatLogKind::Heal, owner, &apl),
        "no APL heal during pull-back; casts {:?}",
        pull.casts()
    );
    let damage: Vec<_> = pull
        .events()
        .iter()
        .filter(|event| {
            event.source == Some(pull.follower.to_bits()) && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert!(
        damage.is_empty(),
        "healer damage {damage:?}; hold still active {}",
        server_combat::followers::holding_pull_damage(pull.harness.app.world(), pull.follower)
    );
    assert!(
        !pull.casts().contains(&19236),
        "legacy defensive bridge ran"
    );
    assert!(
        server_combat::combat_refs::has_active_reference_to(
            pull.harness.app.world(),
            pull.harness.npc,
            owner
        ),
        "waiting must retain the live combat reference"
    );
    assert!(!pull.apl_failed());
}

#[test]
fn arms_casts_damage_beside_engaged_boss_with_pull_back_hold() {
    use shared::components::{Position, PowerType};

    let mut pull = spawn(71);
    let owner = pull.harness.caster;
    let boss = pull.harness.npc;
    let arms = pull.follower;
    let profile = read_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.spec_id == 581)
        .unwrap();
    let tank = spawn_class(
        pull.harness.app.world_mut(),
        owner,
        &profile,
        1,
        0,
        GearSource::Journal,
    )
    .unwrap();
    let boss_position = *pull.harness.app.world().get::<Position>(boss).unwrap();
    let world = pull.harness.app.world_mut();
    world.entity_mut(arms).insert(Position {
        x: boss_position.x + 4.0,
        ..boss_position
    });
    world.entity_mut(tank).insert(Position {
        x: boss_position.x + 1.0,
        ..boss_position
    });
    world
        .entity_mut(boss)
        .insert(server_combat::networking::CombatTarget(Some(tank)));
    let mut pools = world
        .get_mut::<server_combat::spell_cast::powers::PowerPools>(arms)
        .unwrap();
    let rage = pools.get_mut(PowerType::Rage).unwrap();
    rage.current = rage.max;
    server_combat::combat_refs::set_in_combat_with(world, boss, tank);
    server_combat::combat_refs::set_in_combat_with(world, boss, arms);
    server_combat::followers::set_test_pull_back(world, owner, tank);
    assert!(server_combat::followers::holding_pull_damage(world, arms));
    assert!(server_combat::combat_refs::has_active_reference_to(
        world, boss, arms
    ));
    let damage_actions = pull.resolved(
        server_follower_apl::arms_rotation::ACTIONS
            .iter()
            .filter(|(name, _, _)| {
                matches!(
                    *name,
                    "mortal_strike"
                        | "overpower"
                        | "slam"
                        | "rend"
                        | "execute"
                        | "demolish"
                        | "colossus_smash"
                        | "cleave"
                        | "heroic_strike"
                )
            })
            .map(|&(_, spell, _)| spell),
    );
    for _ in 0..100 {
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
        if pull
            .casts()
            .iter()
            .any(|spell| damage_actions.contains(spell))
        {
            assert!(!server_combat::followers::holding_pull_damage(
                pull.harness.app.world(),
                arms
            ));
            return;
        }
    }
    panic!(
        "Arms cast no damaging action within five seconds: {:?}",
        pull.casts()
    );
}

#[test]
fn interrupted_composite_cast_drops_remaining_steps_and_reselects() {
    use server_combat::simc_apl::Action;
    use server_follower_apl::SpecRotation;
    use server_follower_apl::literal_rotation::{PrecombatPoll, execution::Execution};
    use std::sync::{Arc, Mutex};

    struct CompositeProbe(Arc<Mutex<Vec<&'static str>>>);
    impl SpecRotation for CompositeProbe {
        fn action_spells(&self) -> Vec<u32> {
            vec![]
        }
        fn poll_precombat(
            &mut self,
            _: &mut World,
            _: Entity,
            _: &[Entity],
        ) -> Result<PrecombatPoll, String> {
            self.0.lock().unwrap().push("reselect");
            Ok(PrecombatPoll::Finished)
        }
        fn execute(
            &mut self,
            world: &mut World,
            actor: Entity,
            _: Action,
            execution: Execution,
        ) -> Result<(), String> {
            self.0.lock().unwrap().push("execute");
            server_follower_apl::literal_rotation::execution::execute(world, actor, execution)
                .map_err(Into::into)
        }
        fn start_combat_at(&mut self, _: f64) {}
        fn drive(&mut self, _: &mut World, _: Entity, _: &[Entity]) -> Result<(), String> {
            Ok(())
        }
    }

    let mut pull = spawn(259);
    let actor = pull.follower;
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut apl = FollowerApl::new(Box::new(CompositeProbe(log.clone())));
    let action = Action {
        source_row: None,
        name: "apply_poison".into(),
        target: None,
        options: Default::default(),
    };
    apl.execute_plan(
        pull.harness.app.world_mut(),
        actor,
        action,
        Execution::Composite(vec![
            Execution::Spell(2823, actor, None),
            Execution::Spell(3408, actor, None),
        ]),
    )
    .unwrap();
    assert!(
        pull.harness
            .app
            .world()
            .get::<shared::casting::CastState>(actor)
            .is_some(),
        "first poison must start a real cast"
    );
    server_combat::spell_cast::cancel_cast(pull.harness.app.world_mut(), actor);
    for _ in 0..200 {
        pull.harness.app.update();
        let world = pull.harness.app.world();
        let busy = world
            .get::<server_combat::spell_cast::SpellHistory>(actor)
            .is_some_and(|history| {
                history.has_active_gcd(server_combat::spell_cast::history_now(world))
            });
        if !busy {
            break;
        }
    }
    apl.tick(pull.harness.app.world_mut(), actor, &[]).unwrap();
    assert_eq!(*log.lock().unwrap(), ["execute", "reselect"]);
    assert!(!server_combat::auras::has_aura(
        pull.harness.app.world(),
        actor,
        3408
    ));

    let mut control = spawn(259);
    let actor = control.follower;
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut apl = FollowerApl::new(Box::new(CompositeProbe(log.clone())));
    let action = Action {
        source_row: None,
        name: "apply_poison".into(),
        target: None,
        options: Default::default(),
    };
    apl.execute_plan(
        control.harness.app.world_mut(),
        actor,
        action,
        Execution::Composite(vec![
            Execution::Spell(2823, actor, None),
            Execution::Spell(3408, actor, None),
        ]),
    )
    .unwrap();
    for _ in 0..200 {
        control.harness.app.update();
        let world = control.harness.app.world();
        let casting = world.get::<shared::casting::CastState>(actor).is_some();
        let gcd = world
            .get::<server_combat::spell_cast::SpellHistory>(actor)
            .is_some_and(|history| {
                history.has_active_gcd(server_combat::spell_cast::history_now(world))
            });
        if !casting && !gcd {
            break;
        }
    }
    assert!(
        server_combat::auras::has_aura(control.harness.app.world(), actor, 2823),
        "first poison completed"
    );
    apl.tick(control.harness.app.world_mut(), actor, &[])
        .unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        ["execute", "execute"],
        "successful prerequisite continues the composite"
    );
}

#[test]
fn restoration_shaman_maintains_shields_without_enemies() {
    let mut pull = spawn(264);
    let actor = pull.follower;
    let owner = pull.harness.caster;
    let dummy = pull.harness.npc;
    let world = pull.harness.app.world_mut();
    world
        .entity_mut(dummy)
        .insert(server_combat::networking::CombatTarget(None));
    let max = world.get::<Health>(owner).unwrap().max;
    world.entity_mut(owner).insert(Health { current: max, max });
    world
        .entity_mut(actor)
        .insert(server_follower_apl::DirectedOwnerTarget(dummy));
    server_follower_apl::drive_followers(world);
    assert!(world.get::<FollowerApl>(actor).unwrap().in_combat());
    world
        .entity_mut(actor)
        .remove::<server_follower_apl::DirectedOwnerTarget>();
    for _ in 0..200 {
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
    }
    let world = pull.harness.app.world_mut();
    assert!(!world.get::<FollowerAplFailed>(actor).is_some());
    assert!(
        server_combat::auras::has_aura(world, actor, 52127),
        "Water Shield"
    );
    let party = server_follower_apl::healing_party(world, actor);
    assert!(
        party
            .iter()
            .any(|&unit| server_combat::auras::has_aura(world, unit, 974)),
        "Earth Shield"
    );
    server_combat::auras::remove_auras(
        world,
        actor,
        server_combat::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 52127,
    );
    for _ in 0..200 {
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        pull.harness.app.update();
    }
    assert!(
        server_combat::auras::has_aura(pull.harness.app.world(), actor, 52127),
        "expired Water Shield is refreshed"
    );
}

#[test]
fn follower_apl_retries_after_a_pending_navigation_destination() {
    use server_combat::navigation::{Goal, MoveRequest, Navigation, Step};
    use server_combat::simc_apl::{Action, Facts};
    use server_combat::simc_apl_native::{Bindings, Encounter, NativeFacts};
    use server_follower_apl::SpecRotation;
    use server_follower_apl::literal_rotation::{PrecombatPoll, execution::Execution};
    use shared::components::{CreatureMotion, Position};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Probe(Arc<AtomicUsize>);
    impl SpecRotation for Probe {
        fn action_spells(&self) -> Vec<u32> {
            Vec::new()
        }
        fn poll_precombat(
            &mut self,
            _: &mut World,
            _: Entity,
            _: &[Entity],
        ) -> Result<PrecombatPoll, String> {
            Ok(PrecombatPoll::Finished)
        }
        fn execute(
            &mut self,
            _: &mut World,
            _: Entity,
            _: Action,
            _: Execution,
        ) -> Result<(), String> {
            unreachable!()
        }
        fn start_combat_at(&mut self, _: f64) {}
        fn drive(&mut self, world: &mut World, actor: Entity, _: &[Entity]) -> Result<(), String> {
            NativeFacts::new(
                world,
                actor,
                &Bindings::default(),
                Encounter::dungeon(0.0),
                1,
            )
            .value("movement.distance")?;
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    let mut pull = spawn(71);
    let actor = pull.follower;
    let target = pull.harness.npc;
    let runs = Arc::new(AtomicUsize::new(0));
    let world = pull.harness.app.world_mut();
    let from = *world.get::<Position>(actor).unwrap();
    let to = *world.get::<Position>(target).unwrap();
    let map_id = world
        .get::<server_combat::maps::InMap>(actor)
        .unwrap()
        .map_id;
    let mut navigation = Navigation::new(std::path::Path::new("/nonexistent-navigation-fixture"));
    assert!(matches!(
        navigation.advance(
            actor,
            MoveRequest {
                map_id,
                goal: Goal::Chase(target.to_bits()),
                from,
                to,
                distance: 1.0,
                reach: 2.0,
            }
        ),
        Step::Pending
    ));
    world.insert_resource(navigation);
    world.entity_mut(actor).insert((
        CreatureMotion::Run,
        FollowerApl::new(Box::new(Probe(runs.clone()))),
    ));
    server_combat::combat_refs::set_in_combat_with(world, target, actor);
    server_follower_apl::drive_followers(world);
    assert!(
        world.get::<FollowerApl>(actor).is_some(),
        "pending movement must preserve the APL"
    );
    assert!(
        world.get::<FollowerAplFailed>(actor).is_none(),
        "pending movement must not fail the pull"
    );
    assert_eq!(runs.load(Ordering::SeqCst), 0);
    world.entity_mut(actor).insert(CreatureMotion::Still);
    server_follower_apl::drive_followers(world);
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "the same APL must resume on the next ready tick"
    );
}

#[test]
fn follower_apl_keeps_casting_while_its_target_casts() {
    use shared::casting::{CastState, CastType};
    // Murder Row Kystia casts Chaos Barrage 1230298 almost continuously; the Vengeance
    // `disrupt,if=target.debuff.casting.react` row must not stop the APL for the pull.
    let mut pull = spawn(581);
    pull.harness
        .app
        .insert_resource(server_combat::test_support::GroundMap::flat_arena(0.0));
    let target = pull.harness.npc;
    for frame in 0..FRAMES {
        pull.harness
            .app
            .world_mut()
            .entity_mut(target)
            .insert(CastState {
                spell_name: "Chaos Barrage".into(),
                spell_id: 1230298,
                target: pull.follower.to_bits(),
                duration: 3.0,
                elapsed: (frame % 60) as f32 * 0.05,
                cast_type: CastType::Normal,
                interruptible: true,
                pushback_count: 0,
                channel_tick_interval: 0.0,
                channel_tick_timer: 0.0,
                cast_while_moving: false,
            });
        server_combat::followers::update_ai(pull.harness.app.world_mut());
        server_follower_apl::drive_followers(pull.harness.app.world_mut());
        assert!(
            !pull.apl_failed(),
            "target cast stopped the APL at frame {frame}"
        );
        pull.harness.app.update();
    }
    let table = server_follower_apl::vengeance_tank_rotation::ACTIONS;
    let apl = pull.resolved(table.iter().map(|&(_, spell)| spell));
    let casts = pull.casts();
    assert!(
        pull.landed(CombatLogKind::Damage, target, &apl),
        "Vengeance follower casts {casts:?}; no APL damage {apl:?} on the casting target"
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

/// Shadow's APL reads `buff.mind_devourer.react`; its reaction policy must survive
/// the APL reload a new pull performs, or every later pull stops casting.
#[test]
fn class_follower_shadow_apl_keeps_casting_on_the_next_pull() {
    let mut pull = pull(258);
    assert!(
        !pull.apl_failed(),
        "first Shadow pull casts {:?}",
        pull.casts()
    );
    // A pull's end drops its APL; the next pull loads a fresh one.
    let follower = pull.follower;
    pull.harness
        .app
        .world_mut()
        .entity_mut(follower)
        .remove::<FollowerApl>();
    let first = pull.casts().len();
    pull.fight();
    assert!(!pull.apl_failed(), "second Shadow pull stopped its APL");
    assert!(
        pull.casts().len() > first,
        "second Shadow pull casts nothing new: {:?}",
        pull.casts()
    );
}

/// Frost Mage's APL reads `.react` on its tracked buffs (Icicles, Fingers of
/// Frost, Brain Freeze); production configures their acquisition before they exist.
#[test]
fn class_follower_frost_mage_apl_reacts_to_its_procs() {
    let pull = pull(64);
    let tracked: Vec<_> = server_follower_apl::frost_mage_rotation::bindings::BUFFS
        .iter()
        .map(|&(_, spell)| spell)
        .collect();
    assert!(
        pull.landed(CombatLogKind::AuraApplied, pull.follower, &tracked),
        "Frost Mage gained no tracked buff; casts {:?}",
        pull.casts()
    );
    assert!(
        !pull.apl_failed(),
        "Frost Mage APL stopped; casts {:?}",
        pull.casts()
    );
}

/// Shadow Bolt, Hand of Gul'dan, Call Dreadstalkers, Demonbolt, Summon Demonic Tyrant.
const DEMONOLOGY_CORE: [u32; 5] = [686, 105174, 104316, 264178, 265187];
const KEG_SMASH: u32 = 121253;

/// Casts of `spells` in this pull and in the next one, which loads a fresh APL.
fn two_pull_casts(pull: &mut Pull, spells: &[u32]) -> [usize; 2] {
    let count = |pull: &Pull| {
        pull.casts()
            .iter()
            .filter(|spell| spells.contains(spell))
            .count()
    };
    pull.fight();
    assert!(!pull.apl_failed(), "first pull casts {:?}", pull.casts());
    let first = count(pull);
    let follower = pull.follower;
    pull.harness
        .app
        .world_mut()
        .entity_mut(follower)
        .remove::<FollowerApl>();
    pull.fight();
    assert!(!pull.apl_failed(), "second pull casts {:?}", pull.casts());
    [first, count(pull) - first]
}

fn hero_facts(pull: &Pull, names: &[&str]) -> Vec<f64> {
    use server_combat::simc_apl::Facts;
    let bindings = server_follower_apl::demonology_rotation::bindings::build();
    let facts = server_combat::simc_apl_native::NativeFacts::new(
        pull.harness.app.world(),
        pull.follower,
        &bindings,
        server_combat::simc_apl_native::Encounter::dungeon(0.0),
        1,
    );
    names
        .iter()
        .map(|name| facts.value(name).unwrap())
        .collect()
}

const DEMONOLOGY_HERO: [&str; 4] = [
    "hero_tree.diabolist",
    "hero_tree.soul_harvester",
    "talent.diabolic_ritual",
    "talent.demonic_soul",
];

/// SimC APLs assume the hero level: a level-70 Demonology follower has no hero tree,
/// so its hero-gated lists read the profile's logged tree and it keeps casting.
#[test]
fn class_follower_demonology_casts_below_hero_level_on_every_pull() {
    let mut pull = spawn_at(266, 70);
    let hero = hero_facts(&pull, &DEMONOLOGY_HERO);
    assert_eq!(
        hero[0] + hero[1],
        1.0,
        "one logged hero tree reads selected: {hero:?}"
    );
    assert_eq!(
        hero[2] + hero[3],
        1.0,
        "the logged tree's keystone reads enabled: {hero:?}"
    );
    // The live failure cast only the precombat Shadow Bolt each pull.
    let casts = two_pull_casts(&mut pull, &DEMONOLOGY_CORE);
    assert!(
        casts.iter().all(|&count| count >= 2),
        "Demonology core casts per pull {casts:?}; casts {:?}",
        pull.casts()
    );
}

#[test]
fn class_follower_brewmaster_casts_keg_smash_below_hero_level_on_every_pull() {
    let mut pull = spawn_at(268, 70);
    let casts = two_pull_casts(&mut pull, &[KEG_SMASH]);
    assert!(
        casts.iter().all(|&count| count >= 1),
        "Keg Smash per pull {casts:?}; casts {:?}",
        pull.casts()
    );
}

/// At hero level the follower's real selected tree decides; the logged tree is unused.
#[test]
fn class_follower_at_hero_level_reads_its_selected_hero_tree() {
    let mut pull = spawn_at(266, 90);
    let selected = hero_facts(&pull, &DEMONOLOGY_HERO);
    assert_eq!(selected[0] + selected[1], 1.0, "{selected:?}");
    let world = pull.harness.app.world();
    let logged = world
        .get::<server_combat::followers::class_spawn::LoggedHeroTree>(pull.follower)
        .copied()
        .expect("Demonology profile logs a hero tree");
    let other = world
        .resource::<server_combat::spell_info::ClassSpells>()
        .classes
        .iter()
        .flat_map(|class| &class.traits.sub_trees)
        .find(|tree| {
            let name = tree.name.to_lowercase();
            tree.id != logged.0 && (name == "diabolist" || name == "soul harvester")
        })
        .expect("the other Demonology hero tree")
        .id;
    pull.harness
        .app
        .world_mut()
        .entity_mut(pull.follower)
        .insert(server_combat::followers::class_spawn::LoggedHeroTree(other));
    assert_eq!(hero_facts(&pull, &DEMONOLOGY_HERO), selected);
}

/// Raid-haste casts and their lockouts (TC spell_gen_bloodlust).
const LUSTS: [u32; 5] = [2825, 32182, 80353, 390386, 264667];
const ELEMENTAL: u32 = 262;
/// lust_policy.rs INFERRED_WCL constants: p10 live engaged enemies and p10 seconds into the
/// pull at 110 retail trash lusts (lust-audit/report.md).
const TRASH_ENGAGED: usize = 8;
const TRASH_DELAY: f64 = 7.7;
/// Azure Vault 2515 encounter_compat.csv: Leymor, Azureblade, Telash Greywing, Umbrelskul.
const AZURE_BOSSES: [(u32, u32); 4] = [(186644, 0), (186739, 1), (186737, 2), (186738, 3)];

impl Pull {
    fn step(&mut self) {
        server_combat::followers::update_ai(self.harness.app.world_mut());
        server_follower_apl::drive_followers(self.harness.app.world_mut());
        self.harness.app.update();
    }

    fn lusts(&self) -> usize {
        self.casts()
            .iter()
            .filter(|spell| LUSTS.contains(spell))
            .count()
    }

    fn in_combat(&self) -> bool {
        self.harness
            .app
            .world()
            .get::<FollowerApl>(self.follower)
            .is_some_and(FollowerApl::in_combat)
    }

    fn copy(&self) -> server_combat::maps::InMap {
        *self
            .harness
            .app
            .world()
            .get::<server_combat::maps::InMap>(self.harness.caster)
            .unwrap()
    }

    /// A durable enemy of `entry` fighting the owner, beside the harness dummy.
    fn spawn_enemy(&mut self, entry: u32, x: f32) -> Entity {
        let (owner, copy) = (self.harness.caster, self.copy());
        let template = self
            .harness
            .app
            .world()
            .resource::<server_combat::world_data::WorldData>()
            .templates[&entry]
            .clone();
        let enemy = server_combat::spell_cast::test_harness::spawn_npc(&mut self.harness.app, x);
        let world = self.harness.app.world_mut();
        world.entity_mut(enemy).insert((
            shared::components::Npc {
                template_id: entry,
                name: template.name.clone(),
            },
            server_combat::zones::CreatureFlagsExtra(template.flags_extra),
            copy,
            Health {
                current: 10_000_000.0,
                max: 10_000_000.0,
            },
            server_combat::networking::CombatTarget(Some(owner)),
        ));
        server_combat::combat_refs::set_in_combat_with(world, enemy, owner);
        enemy
    }

    /// The harness dummy plus `count - 1` Kobold Vermin, all fighting the owner.
    fn engage_pack(&mut self, count: usize) -> Vec<Entity> {
        let mut pack = vec![self.harness.npc];
        for index in 1..count {
            pack.push(self.spawn_enemy(6, 1.0 + index as f32 * 0.5));
        }
        pack
    }

    /// End the pull: remove `enemies` and step until the follower leaves combat.
    fn end_pull(&mut self, enemies: &[Entity]) {
        for &enemy in enemies {
            self.harness.app.world_mut().despawn(enemy);
        }
        for _ in 0..100 {
            self.step();
            if !self.in_combat() {
                return;
            }
        }
        panic!("follower stayed in combat after its enemies were removed");
    }

    /// Advance 600 s out of combat in one frame: the Sated/Exhaustion lockouts and the 300 s
    /// lust cooldown lapse.
    fn wait_out_lust_lockouts(&mut self) {
        use std::time::Duration;
        let app = &mut self.harness.app;
        let step = Duration::from_secs(600);
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(step);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(step));
        app.update();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
        let world = app.world_mut();
        let party = server_combat::followers::party_units(world, self.harness.caster);
        assert!(
            !party
                .into_iter()
                .any(|member| server_combat::auras::bloodlust::has_lockout(world, member))
        );
    }

    fn now(&self) -> f64 {
        server_combat::spell_cast::now(self.harness.app.world())
    }

    /// Step until the follower lusts or `frames` pass; the world time of the lust.
    fn run_until_lust(&mut self, frames: usize) -> Option<f64> {
        let before = self.lusts();
        for _ in 0..frames {
            self.step();
            if self.lusts() > before {
                return Some(self.now());
            }
        }
        None
    }
}

fn lust_pull() -> Pull {
    let mut pull = spawn(ELEMENTAL);
    let path = server_combat::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    // The copy's instance script reads its boss bindings from the map store.
    let maps = server_combat::maps::store::load_map_store(&db).unwrap();
    pull.harness.app.insert_resource(maps);
    pull
}

#[test]
fn follower_lust_skips_trash_below_retail_engaged_count() {
    let mut pull = lust_pull();
    pull.engage_pack(TRASH_ENGAGED - 1);
    assert_eq!(pull.run_until_lust(400), None, "casts {:?}", pull.casts());
    assert!(pull.in_combat());
}

#[test]
fn follower_lusts_large_trash_after_retail_delay() {
    let mut pull = lust_pull();
    pull.engage_pack(TRASH_ENGAGED);
    let pulled = pull.now();
    let into = pull
        .run_until_lust(400)
        .unwrap_or_else(|| panic!("no lust on a large pull: {:?}", pull.casts()))
        - pulled;
    // The follower's combat clock starts on its first engaged frame.
    assert!(
        (TRASH_DELAY - 0.1..TRASH_DELAY + 1.0).contains(&into),
        "lust {into} s into the pull"
    );
}

#[test]
fn follower_holds_its_next_lust_for_the_final_boss() {
    let mut pull = lust_pull();
    let pack = pull.engage_pack(TRASH_ENGAGED);
    assert!(pull.run_until_lust(400).is_some(), "first lust on trash");
    pull.end_pull(&pack);
    pull.wait_out_lust_lockouts();
    // A later large trash pull after the lockout ends keeps the lust.
    let pack = (0..TRASH_ENGAGED)
        .map(|index| pull.spawn_enemy(6, 1.0 + index as f32 * 0.5))
        .collect::<Vec<_>>();
    assert_eq!(pull.run_until_lust(400), None, "trash after expiry");
    pull.end_pull(&pack);
    // Umbrelskul is not the final boss while another Azure Vault boss is alive; its
    // template carries no creature boss flag, only the copy's encounter binding.
    let copy = pull.copy();
    let boss = pull.spawn_enemy(AZURE_BOSSES[3].0, 2.0);
    let pulled = pull.now();
    assert_eq!(pull.run_until_lust(300), None, "non-final boss");
    for &(_, boss_id) in &AZURE_BOSSES[..3] {
        assert!(server_combat::scripted_ai::instance_script::set_boss_state(
            pull.harness.app.world_mut(),
            copy,
            boss_id,
            server_combat::scripted_ai::instance_script::EncounterState::Done,
        ));
    }
    // The final boss lusts within the same pull, past 10 s (no early-pull cap).
    let into = pull
        .run_until_lust(40)
        .unwrap_or_else(|| panic!("no final-boss lust: {:?}", pull.casts()))
        - pulled;
    assert!(into > 10.0, "boss lust {into} s into the pull");
    assert!(pull.harness.app.world().get_entity(boss).is_ok());
}

#[test]
fn fixed_length_fight_never_lusts_from_the_follower_policy() {
    // Its lust is the SimC reference's override.bloodlust, applied by the harness.
    let mut pull = lust_pull();
    pull.harness
        .app
        .insert_resource(server_follower_apl::registry::FixedLengthFight { duration: 300.0 });
    pull.engage_pack(TRASH_ENGAGED);
    assert_eq!(pull.run_until_lust(400), None, "casts {:?}", pull.casts());
    assert!(pull.in_combat());
}
