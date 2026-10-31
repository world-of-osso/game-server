use super::*;
use crate::followers::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::stats::{EquippedItems, NativeItemInstances};
use rusqlite::{Connection, OpenFlags};

#[test]
fn story_follower_does_not_gain_mana_from_player_only_aura() {
    let (mut h, unit, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    assert!(h.app.world().get::<CharacterVisual>(unit).is_none());
    h.app.world_mut().entity_mut(unit).insert((
        Level(90),
        Mana {
            current: 1.0,
            max: 1.0,
        },
    ));
    h.app.update();
    let baseline = h.app.world().get::<Mana>(unit).unwrap().max;
    assert!(baseline > 1.0);
    let caster = h.caster;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: unit,
            spell_id: 210126,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), unit, 210126));
    h.app.update();
    assert_eq!(h.app.world().get::<Mana>(unit).unwrap().max, baseline);
}

#[test]
fn class_follower_spawn_has_visual_real_gear_progression_and_no_story_kit() {
    let (mut h, _, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    let source = crate::test_data::world_data();
    let mut world_data = h
        .app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>();
    world_data.class_level_base_hp = source.class_level_base_hp.clone();
    world_data.class_level_base_mana = source.class_level_base_mana.clone();
    drop(world_data);
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(super::super::catalog::load_catalog(&db).unwrap());
    h.app.insert_resource(
        super::super::gear::load_source(&path, &path.parent().unwrap().join("db2/active")).unwrap(),
    );
    h.app
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    h.app.insert_resource(NativeItemInstances {
        bonuses: crate::item_instance::load::load_bonus_tables(&db).unwrap(),
        enchants: crate::item_instance::load::load_enchantment_tables(&db).unwrap(),
        effects: crate::item_instance::load::load_item_effect_tables(&db).unwrap(),
    });
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut instances = crate::maps::InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            2515,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 205,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_792_454_400,
            },
        )
        .unwrap()
        .map;
    h.app.insert_resource(instances);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((copy, Level(90)));
    let profiles = super::super::class_profiles::read_profiles().unwrap();
    for (slot, spec) in [581, 73, 66, 250, 268, 104, 257, 62]
        .into_iter()
        .enumerate()
    {
        let profile = profiles
            .iter()
            .find(|profile| profile.spec_id == spec)
            .unwrap();
        let unit = spawn_class(
            h.app.world_mut(),
            h.caster,
            profile,
            1,
            slot,
            GearSource::Journal,
        )
        .unwrap();
        let world = h.app.world();
        assert!(world.get::<shared::components::Player>(unit).is_none());
        assert!(
            world
                .get::<shared::components::ModelDisplay>(unit)
                .is_none()
        );
        assert!(world.get::<super::super::AplControlled>(unit).is_some());
        let visual = world.get::<CharacterVisual>(unit).unwrap();
        assert_eq!(
            (visual.race, visual.class, visual.appearance.sex),
            (profile.race, profile.class_id, 1)
        );
        let follower = world.get::<Follower>(unit).unwrap();
        assert_eq!(
            (follower.profile.spec_id, follower.profile.entry),
            (spec, CLASS_FOLLOWER_ENTRY)
        );
        assert_eq!(follower.profile.cooldown_spell, 0);
        assert_eq!(follower.profile.heal_spell, 0);
        let expected_taunt = match spec {
            581 => 185245,
            73 => 355,
            66 => 62124,
            250 => 56222,
            268 => 115546,
            104 => 6795,
            _ => 0,
        };
        assert_eq!(follower.profile.taunt_spell, expected_taunt);
        assert_eq!(follower.profile.form_spell, 0);
        assert_eq!(
            world.get::<shared::components::ActiveSpec>(unit).unwrap().0,
            spec
        );
        assert!(
            world
                .get::<crate::class_progression::KnownSpells>(unit)
                .unwrap()
                .iter()
                .count()
                > 0
        );
        assert!(
            world
                .get::<crate::class_progression::TraitLoadout>(unit)
                .unwrap()
                .active_talent_spells()
                .count()
                > 0
        );
        assert!(
            world
                .get::<crate::spell_cast::powers::PowerPools>(unit)
                .is_some()
        );
        let physical = world.get::<EquippedItems>(unit).unwrap();
        let visible = world
            .get::<shared::components::EquipmentAppearance>(unit)
            .unwrap();
        assert!(!physical.items.is_empty());
        assert!(!visible.entries.is_empty());
        assert!(visible.entries.iter().all(|entry| {
            physical.items.iter().any(|(_, source, item)| {
                Some(*item) == entry.item_id && Some(*source) == entry.definition_source
            })
        }));
        if !follower.profile.melee {
            let attack = follower.profile.attack_spell;
            let classes = world.resource::<crate::spell_info::ClassSpells>();
            let spec_data = classes
                .classes
                .iter()
                .find(|class| class.class_id == profile.class_id)
                .unwrap()
                .specs
                .iter()
                .find(|entry| entry.spec_id == spec)
                .unwrap();
            assert!(
                spec_data
                    .spells
                    .iter()
                    .any(|spell| spell.spell_id == attack)
            );
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(attack)
                .unwrap();
            assert!(
                spell
                    .misc
                    .as_ref()
                    .and_then(|misc| misc.range)
                    .is_some_and(|range| range.max_hostile.max(range.max_friendly) >= 20.0)
            );
            assert!(!spell.effects.iter().any(|effect| matches!(
                effect.effect,
                crate::death::resurrection::EFFECT_RESURRECT
                    | crate::death::resurrection::EFFECT_RESURRECT_WITH_AURA
            )));
        }
        let items = world.get::<EquippedItems>(unit).unwrap().clone();
        let modifiers = world
            .get::<crate::stats::EquippedItemModifiers>(unit)
            .unwrap()
            .clone();
        let known = world
            .get::<crate::class_progression::KnownSpells>(unit)
            .unwrap()
            .clone();
        let traits = world
            .get::<crate::class_progression::TraitLoadout>(unit)
            .unwrap()
            .clone();
        let combat = world
            .get::<shared::components::CombatStatus>(unit)
            .copied()
            .unwrap_or_default();
        let (known_entries, _, _) = crate::class_progression::evaluate_follower_progression(
            world.resource::<crate::spell_info::ClassSpells>(),
            profile.class_id,
            profile.race,
            spec,
            90,
            profile.talent_source().unwrap(),
            profile.talent_entries.as_ref().unwrap(),
        )
        .unwrap();
        assert_eq!(
            known.iter().collect::<Vec<_>>(),
            known_entries
                .iter()
                .map(|spell| spell.spell_id)
                .collect::<Vec<_>>()
        );
        let passives: Vec<_> = known_entries
            .iter()
            .filter(|spell| spell.passive)
            .map(|spell| spell.spell_id)
            .collect();
        let player =
            crate::spell_cast::test_harness::spawn_player(&mut h.app, profile.class_id, 0.0);
        h.app
            .world_mut()
            .entity_mut(player)
            .remove::<crate::stats::CharacterStats>()
            .insert((
                shared::components::Player {
                    name: format!("Mirror{slot}"),
                    race: profile.race,
                    class: profile.class_id,
                    appearance: CharacterAppearance {
                        sex: 1,
                        ..Default::default()
                    },
                },
                copy,
                Level(90),
                UnitLevel(90),
                shared::components::ActiveSpec(spec),
                items,
                modifiers,
                known,
                traits,
                combat,
                shared::components::UnitStats::default(),
                shared::components::CombatRatings::default(),
                shared::components::DerivedStats::default(),
                shared::components::Health {
                    current: 1.0,
                    max: 1.0,
                },
                Mana {
                    current: 1.0,
                    max: 1.0,
                },
            ));
        crate::auras::sync_passives(
            h.app.world_mut(),
            player,
            crate::auras::AuraSource::KnownSpell,
            &passives,
        );
        h.app.update();
        let world = h.app.world();
        let follower_stats = world.get::<crate::stats::CharacterStats>(unit).unwrap();
        let player_stats = world.get::<crate::stats::CharacterStats>(player).unwrap();
        assert_eq!(follower_stats.primary, player_stats.primary);
        assert_eq!(
            world.get::<shared::components::Health>(unit),
            world.get::<shared::components::Health>(player)
        );
        assert_eq!(world.get::<Mana>(unit), world.get::<Mana>(player));
        if spec == 581 {
            assert!(crate::auras::has_aura(world, unit, 189926));
            let enemy = h.npc;
            let owner = h.caster;
            let position = *world.get::<shared::components::Position>(unit).unwrap();
            h.app.world_mut().entity_mut(enemy).insert((
                copy,
                position,
                shared::components::ThreatTable::default(),
            ));
            h.app.update();
            crate::creature_aggro::add_school_threat(h.app.world_mut(), enemy, unit, 100.0, 1);
            let threat = h
                .app
                .world()
                .get::<shared::components::ThreatTable>(enemy)
                .unwrap();
            let amount = threat
                .entries()
                .iter()
                .find(|entry| entry.entity == unit.to_bits())
                .unwrap()
                .threat;
            assert_eq!(
                amount, 1050.0,
                "Vengeance's passive must multiply native threat"
            );
            crate::combat::combat_stop(h.app.world_mut(), unit);
            h.app
                .world_mut()
                .entity_mut(enemy)
                .remove::<crate::creature_aggro::CurrentVictim>()
                .insert((
                    shared::components::ThreatTable::default(),
                    crate::creature_ai::CreatureState::Idle,
                ));
            crate::creature_aggro::attack_start(h.app.world_mut(), enemy, owner);
            assert_eq!(
                h.app
                    .world()
                    .get::<crate::networking::CombatTarget>(enemy)
                    .unwrap()
                    .0,
                Some(owner),
                "enemy must first attack the owner"
            );
            let party = super::super::ai::party_units(h.app.world_mut(), owner);
            let follower = h.app.world().get::<Follower>(unit).unwrap().clone();
            let enemies =
                super::super::ai::engaged_enemies(h.app.world_mut(), &follower, unit, &party);
            assert!(
                enemies.iter().any(|(candidate, _)| *candidate == enemy),
                "owner's attacker must be available for pickup: {enemies:?}"
            );
            for _ in 0..100 {
                super::super::update_ai(h.app.world_mut());
                h.advance(1);
                if h.app
                    .world()
                    .get::<crate::networking::CombatTarget>(enemy)
                    .and_then(|target| target.0)
                    == Some(unit)
                {
                    break;
                }
            }
            assert!(
                h.inbox.go.iter().any(|event| event.caster == unit.to_bits()
                    && event.spell_id == 185245
                    && event.target == Some(enemy.to_bits())),
                "Vengeance must cast native Torment against the owner's attacker: {:?}",
                h.inbox.go
            );
            assert_eq!(
                h.app
                    .world()
                    .get::<crate::networking::CombatTarget>(enemy)
                    .unwrap()
                    .0,
                Some(unit),
                "Torment must recover the enemy"
            );
        }
    }
}
