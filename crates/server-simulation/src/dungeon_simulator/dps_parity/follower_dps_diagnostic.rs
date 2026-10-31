//! Production follower gear and AI, measured independently of parity drivers.
use super::*;
use shared::components::{Level, UnitLevel, UnitSummonedBy};
use shared::protocol::CombatLogKind;

fn build_follower_app(path: &std::path::Path) -> App {
    let mut app = super::super::runtime::build_app_with_world_db(path);
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    app.insert_resource(crate::followers::simulator::load_parity_catalog(&db).unwrap());
    crate::spell_cast::area_trigger::install_native_catalog(app.world_mut(), &db).unwrap();
    app
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_dps_loads_spell_and_item_catalogs_from_selected_database() {
    let source = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        std::path::Path::new(&source).canonicalize().unwrap(),
        "startup catalogs must read the requested snapshot"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world.db");
    std::fs::copy(source, &path).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "UPDATE spell_effect SET EffectBasePointsF=777 WHERE SpellID=391142 AND EffectIndex=0 AND DifficultyID=0",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO item_set(ID, SetFlags, RequiredSkill, RequiredSkillRank) VALUES (9999999, 0, 0, 0)",
        [],
    )
    .unwrap();
    drop(db);
    let app = build_follower_app(&path);
    let spell = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(391142)
        .unwrap();
    assert_eq!(
        spell
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .base_points,
        777.0
    );
    assert!(
        app.world()
            .resource::<crate::item_proc_data::ItemProcInputs>()
            .sets
            .contains_key(&9999999)
    );
}

fn prepare_follower(
    entry: u32,
    level: u8,
    map: u32,
) -> (App, Entity, Entity, crate::followers::Profile) {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB")
        .expect("FOLLOWER_DPS_WORLD_DB must name the measurement snapshot");
    let mut app = build_follower_app(std::path::Path::new(&path));
    let copy = app
        .world_mut()
        .resource_scope(|world, maps: Mut<crate::maps::MapStore>| {
            world
                .resource_mut::<crate::maps::InstanceManager>()
                .create_map(
                    &maps,
                    map,
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
                .map
        });
    app.world_mut().remove_resource::<super::super::Space>();
    crate::simc_apl_native::install_open_arena(app.world_mut());
    let profile = app
        .world()
        .resource::<crate::followers::FollowerCatalog>()
        .profiles
        .iter()
        .find(|profile| profile.entry == entry)
        .unwrap()
        .clone();
    let actor = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        profile.spec_id,
        level,
        copy,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .profile
            .entry,
        entry
    );
    crate::followers::gear::equip(app.world_mut(), actor, &profile).unwrap();
    let target = spawn_dummy(&mut app, actor, 0).unwrap();
    let armor = app
        .world()
        .resource::<crate::world_data::WorldData>()
        .creature_armor_at(DUMMY_ENTRY, 205, level)
        .expect("native dummy armor at target level");
    app.world_mut().entity_mut(target).insert((
        Level(level),
        UnitLevel(level),
        crate::level_scaling::CreatureArmor(armor),
    ));
    app.world_mut()
        .entity_mut(target)
        .remove::<shared::level_scaling::LevelScaling>();
    (app, actor, target, profile)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB and FOLLOWER_DPS_TANK_TRACE"]
fn follower_tank_precombat_starts_consecration_on_simc_clock() {
    let (mut app, actor, target, _) = prepare_follower(209057, 90, 2813);
    app.world_mut()
        .entity_mut(actor)
        .insert(server::test_support::LiteralParityFixture);
    for _ in 0..5 {
        app.update();
    }
    let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE").expect("tank trace");
    super::follower_tank_precombat::execute(&mut app, actor, target, std::path::Path::new(&trace))
        .unwrap();
    assert!(crate::spell_cast::scripts::has_active_consecration(
        app.world_mut(),
        actor
    ));
    assert!(
        crate::auras::has_aura(app.world(), target, 204242),
        "precombat cast applies its target debuff"
    );
    let spell = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(26573)
        .unwrap();
    let history = app
        .world()
        .get::<crate::spell_cast::SpellHistory>(actor)
        .unwrap();
    let now = crate::spell_cast::now(app.world());
    assert!(!history.has_active_gcd(now));
    assert_eq!(
        crate::spell_cast::powers::power_value(
            app.world(),
            actor,
            shared::components::PowerType::HolyPower
        ),
        Some((1.0, 5.0))
    );
    let remaining = history.remaining_cooldown_ms(spell, now);
    assert!(
        (1500..=2200).contains(&remaining),
        "precombat cooldown {remaining}ms"
    );
    crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
    let pulses = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(81297)
                    && event.kind == CombatLogKind::Damage
            })
            .count()
    };
    for _ in 0..19 {
        app.update();
    }
    assert_eq!(
        pulses(&app),
        0,
        "precombat field cannot pulse before one second"
    );
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        pulses(&app),
        1,
        "precombat field starts with one immediate pulse"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB and a Protection Warrior SimC trace"]
fn follower_warrior_precombat_casts_battle_stance() {
    let (mut app, actor, target, profile) = prepare_follower(215517, 90, 2813);
    assert_eq!(profile.spec_id, 73);
    app.world_mut()
        .entity_mut(actor)
        .insert(server::test_support::LiteralParityFixture);
    for _ in 0..5 {
        app.update();
    }
    let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE").expect("warrior tank trace");
    super::follower_tank_precombat::execute(&mut app, actor, target, std::path::Path::new(&trace))
        .unwrap();
    assert!(crate::auras::has_aura(app.world(), actor, 386164));
    assert!(
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(386164)
                && event.kind == CombatLogKind::CastSuccess)
    );
}

#[test]
#[ignore = "requires MID2 Blood inputs and a Blood Death Knight SimC trace"]
fn blood_tank_precombat_casts_deaths_caress() {
    let (profile, traits) = super::read_reference(250).unwrap();
    let race = super::inputs::read_reference_race(&profile.race).unwrap();
    let mut run = super::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race },
    )
    .unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let actor = run.actor;
    let target = run.targets[0];
    let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE").expect("Blood tank trace");
    super::follower_tank_precombat::execute(
        &mut run.app,
        actor,
        target,
        std::path::Path::new(&trace),
    )
    .unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(195292)
                && event.kind == CombatLogKind::CastSuccess)
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(195292)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(crate::auras::has_aura(run.app.world(), target, 55078));
}

#[test]
#[ignore = "requires MID2 Vengeance inputs and a Vengeance Demon Hunter SimC trace"]
fn vengeance_tank_precombat_casts_sigil_and_immolation_aura() {
    let (profile, traits) = super::read_reference(581).unwrap();
    let race = super::inputs::read_reference_race(&profile.race).unwrap();
    let mut run = super::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race },
    )
    .unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let actor = run.actor;
    let target = run.targets[0];
    let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE").expect("Vengeance tank trace");
    super::follower_tank_precombat::execute(
        &mut run.app,
        actor,
        target,
        std::path::Path::new(&trace),
    )
    .unwrap();
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [204596, 258920] {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::CastSuccess)
        );
    }
    assert!(crate::auras::has_aura(run.app.world(), actor, 258920));
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(204598)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB and FOLLOWER_DPS_TANK_TRACE"]
fn follower_tank_replay_survives_lethal_hit_and_keeps_avoidance_procs() {
    use server::test_support::healer_intake::{Hit, HitKind};
    use shared::components::Health;
    let _seed = crate::random::seed_scope(70448);
    let (mut app, actor, source, _) = prepare_follower(209057, 90, 2813);
    app.world_mut()
        .entity_mut(actor)
        .insert(server::test_support::LiteralParityFixture);
    for _ in 0..5 {
        app.update();
    }
    let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE").expect("tank trace");
    let mut scenario = super::follower_tank_scenario::TankScenario::read(
        std::path::Path::new(&trace),
        app.world_mut(),
        source,
        actor,
        90,
    )
    .unwrap();
    scenario.advance(app.world_mut(), 2000).unwrap();
    let hit = |outcome: &str| Hit {
        tick: 40,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 10_000_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: outcome.into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(app.world_mut(), source, actor, &hit("Hit")).unwrap();
    assert_eq!(app.world().get::<Health>(actor).unwrap().current, 1.0);
    assert!(
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(source.to_bits())
                && event.target == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(crate::auras::has_aura(app.world(), actor, 85043));
    let shield = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(31935)
        .unwrap()
        .clone();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 31935, Some(source), false).unwrap();
    let remaining = |app: &App| {
        app.world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .remaining_cooldown_ms(&shield, crate::spell_cast::now(app.world()))
    };
    assert!(remaining(&app) > 0);
    let mut reset = false;
    for _ in 0..40 {
        crate::combat::replay_healer_hit(app.world_mut(), source, actor, &hit("Dodge")).unwrap();
        if remaining(&app) == 0 {
            reset = true;
            break;
        }
        for _ in 0..21 {
            app.update();
        }
    }
    assert!(
        reset,
        "avoidance after lethal damage must still trigger Grand Crusader"
    );
    assert!(app.world().get::<Health>(actor).unwrap().current > 0.0);
}

#[test]
#[ignore = "production follower single-target throughput diagnostic"]
fn follower_dps_production_dummy() {
    let entry: u32 = std::env::var("FOLLOWER_DPS_ENTRY")
        .unwrap()
        .parse()
        .unwrap();
    let level: u8 = std::env::var("FOLLOWER_DPS_LEVEL")
        .unwrap()
        .parse()
        .unwrap();
    let map: u32 = std::env::var("FOLLOWER_DPS_MAP").unwrap().parse().unwrap();
    let seed: u64 = std::env::var("FOLLOWER_DPS_SEED").unwrap().parse().unwrap();
    let _seed = crate::random::seed_scope(seed);
    let (mut app, actor, target, profile) = prepare_follower(entry, level, map);
    let armor = app
        .world()
        .get::<crate::level_scaling::CreatureArmor>(target)
        .unwrap()
        .0;
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    let mut tank_scenario = if profile.role == shared::group::GroupRole::Tank {
        let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE")
            .expect("tank measurements require a SimC tank dummy trace");
        super::follower_tank_precombat::execute(
            &mut app,
            actor,
            target,
            std::path::Path::new(&trace),
        )
        .expect("execute SimC tank precombat");
        Some(
            super::follower_tank_scenario::TankScenario::read(
                std::path::Path::new(&trace),
                app.world_mut(),
                target,
                actor,
                level,
            )
            .expect("load SimC tank dummy trace"),
        )
    } else {
        None
    };
    app.world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(target)));
    app.world_mut()
        .entity_mut(owner)
        .insert(crate::combat::AutoAttack::new(target));
    crate::combat_refs::set_in_combat_with(app.world_mut(), owner, target);
    let stats = app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    let sheet = json!({"primary": stats.primary, "spell_power": stats.spell_power, "attack_power": stats.attack_power,
        "ranged_attack_power":stats.ranged_attack_power,"weapon_attack_power":format!("{:?}", stats.weapon_attack_power), "weapon":format!("{:?}", stats.main_hand),
        "crit_pct": stats.crit_pct, "haste_pct": stats.haste_pct, "mastery_pct": stats.mastery_pct,
        "versatility_pct": stats.versatility_pct});
    let items = format!(
        "{:?}",
        app.world()
            .get::<crate::stats::EquippedItems>(actor)
            .unwrap()
            .items
    );
    let mut known_spells: Vec<_> = app
        .world()
        .get::<crate::class_progression::KnownSpells>(actor)
        .unwrap()
        .iter()
        .collect();
    known_spells.sort_unstable();
    let talents = format!(
        "{:?}",
        app.world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap()
    );
    let modifiers = format!(
        "{:?}",
        app.world()
            .get::<crate::stats::EquippedItemModifiers>(actor)
            .unwrap()
            .items
    );
    app.world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    let pet_factor = std::env::var_os("FOLLOWER_FACTOR_PET").is_some();
    if let Ok(spell) = std::env::var("FOLLOWER_FACTOR_SPELL")
        && !pet_factor
    {
        let spell = spell.parse().expect("numeric FOLLOWER_FACTOR_SPELL");
        app.insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor, target, spell, "follower", 128,
            ),
        );
    }
    let mut casting = 0u32;
    let mut gcd_only = 0u32;
    let mut gcd_total = 0u32;
    let mut ready = 0u32;
    let mut sources = std::collections::BTreeSet::from([actor.to_bits()]);
    let mut checkpoints = Vec::new();
    let mut flame_shock_frames = 0u32;
    let mut lava_surge_frames = 0u32;
    let mut storm_frames = 0u32;
    let mut lava_during_storm = 0u32;
    let mut bolt_during_storm = 0u32;
    let mut stable_storm_frames = 0u32;
    let mut lava_during_stable_storm = 0u32;
    let mut seen_events = 0usize;
    let mut cast_trace = Vec::new();
    let trace_actions = std::env::var_os("FOLLOWER_TRACE_ACTIONS").is_some()
        || std::env::var_os("FOLLOWER_VERIFY_WINGS_ORDER").is_some();
    let mut stat_sums = [0.0f64; 6];
    for frame in 0..2400 {
        if let Some(scenario) = &mut tank_scenario {
            scenario
                .advance(app.world_mut(), (frame + 1) * 50)
                .expect("replay SimC tank pressure");
        }
        // SimC fixed-time Patchwerk uses a 100% to 0% target-health timeline.
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * (2400 - frame) as f32 / 2400.0;
        let before_power = crate::spell_cast::powers::power_value(
            app.world(),
            actor,
            shared::components::PowerType::HolyPower,
        );
        app.update();
        let world = app.world();
        let now = crate::spell_cast::history_now(world);
        let stats = world.get::<crate::stats::CharacterStats>(actor).unwrap();
        for (sum, value) in stat_sums.iter_mut().zip([
            stats.primary.intellect,
            stats.spell_power,
            stats.crit_pct,
            stats.haste_pct,
            stats.mastery_pct,
            stats.versatility_pct,
        ]) {
            *sum += f64::from(value);
        }
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(profile.attack_spell)
            .unwrap();
        let gcd = world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .is_some_and(|history| history.on_gcd(spell, now));
        if gcd {
            gcd_total += 1;
        }
        if world.get::<shared::casting::CastState>(actor).is_some() {
            casting += 1;
        } else if gcd {
            gcd_only += 1;
        } else {
            ready += 1;
        }
        if frame % 200 == 199 {
            let pools = format!(
                "{:?}",
                world.get::<crate::spell_cast::powers::PowerPools>(actor)
            );
            let health = world.get::<shared::components::Health>(target).unwrap();
            checkpoints.push(json!({"at_s": (frame + 1) as f64 / 20.0, "pools": pools, "target_health": health.current,
                "actor_health": world.get::<shared::components::Health>(actor).map(|h| (h.current, h.max)),
                "owner_health": world.get::<shared::components::Health>(owner).map(|h| (h.current, h.max)),
                "actor_combat": world.get::<shared::components::CombatStatus>(actor).map(|s| s.0),
                "owner_combat": world.get::<shared::components::CombatStatus>(owner).map(|s| s.0),
                "actor_position": format!("{:?}", world.get::<shared::components::Position>(actor)),
                "owner_position": format!("{:?}", world.get::<shared::components::Position>(owner))}));
        }
        flame_shock_frames += u32::from(crate::auras::has_aura(world, target, 188389));
        lava_surge_frames += u32::from(crate::auras::has_aura(world, actor, 77762));
        let mut storm_query = app.world_mut().query::<(
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>();
        let storm = storm_query.iter(app.world()).any(|(summon, npc)| {
            summon.owner == actor
                && matches!(npc.template_id, 77936 | 77942)
                && summon.expires_at.is_some_and(|expiry| expiry > now)
        });
        storm_frames += u32::from(storm);
        stable_storm_frames = if storm { stable_storm_frames + 1 } else { 0 };
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if trace_actions {
            let after_power = crate::spell_cast::powers::power_value(
                app.world(),
                actor,
                shared::components::PowerType::HolyPower,
            );
            for event in &events[seen_events..] {
                if event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
                {
                    cast_trace.push(
                        json!({"at_s": (frame + 1) as f64 / 20.0, "spell": event.spell_id,
                        "power_before": before_power, "power_after": after_power,
                        "valor": crate::auras::has_aura(app.world(), actor, 1269179),
                        "divine_purpose": crate::auras::has_aura(app.world(), actor, 223819)}),
                    );
                }
            }
        }
        if storm {
            lava_during_storm += events[seen_events..]
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(51505)
                        && event.kind == CombatLogKind::CastStart
                })
                .count() as u32;
            if stable_storm_frames > 60 {
                lava_during_stable_storm += events[seen_events..]
                    .iter()
                    .filter(|event| {
                        event.source == Some(actor.to_bits())
                            && event.spell_id == Some(51505)
                            && event.kind == CombatLogKind::CastSuccess
                    })
                    .count() as u32;
            }
            bolt_during_storm += events[seen_events..]
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(188196)
                        && event.kind == CombatLogKind::CastStart
                })
                .count() as u32;
        }
        seen_events = events.len();
        if pet_factor
            && app
                .world()
                .get_resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
                .is_none()
        {
            let mut pets = app
                .world_mut()
                .query::<(Entity, &crate::hunter_pet::HunterPet)>();
            let pet = pets
                .iter(app.world())
                .find(|(_, pet)| pet.owner == actor && pet.slot == 0)
                .map(|(entity, _)| entity);
            if let Some(pet) = pet {
                let spell = std::env::var("FOLLOWER_FACTOR_SPELL")
                    .unwrap()
                    .parse()
                    .unwrap();
                app.insert_resource(
                    crate::spell_cast::effects::factor_capture::FactorCapture::new(
                        pet, target, spell, "pet", 128,
                    ),
                );
            }
        }
        let mut query = app.world_mut().query::<(Entity, &UnitSummonedBy)>();
        for (entity, owner) in query.iter(app.world()) {
            if sources.contains(&owner.0) {
                sources.insert(entity.to_bits());
            }
        }
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mut damage = BTreeMap::<u32, i64>::new();
    let mut hits = BTreeMap::<u32, usize>::new();
    let mut hit_breakdown = BTreeMap::<u32, [i64; 4]>::new();
    let mut damage_sources = BTreeMap::<String, usize>::new();
    let mut casts = BTreeMap::<u32, usize>::new();
    let mut aura_applications = BTreeMap::<u32, usize>::new();
    let mut energize = BTreeMap::<u32, i64>::new();
    let incoming: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage && event.target == Some(actor.to_bits())
        })
        .collect();
    for event in events {
        if !event.source.is_some_and(|id| sources.contains(&id)) {
            continue;
        }
        let id = event.spell_id.unwrap_or(0);
        if event.kind == CombatLogKind::Damage && event.target == Some(target.to_bits()) {
            *damage.entry(id).or_default() += i64::from(event.amount);
            *hits.entry(id).or_default() += 1;
            let bucket = hit_breakdown.entry(id).or_default();
            let index = if event.crit { 2 } else { 0 };
            bucket[index] += 1;
            bucket[index + 1] += i64::from(event.amount);
            *damage_sources
                .entry(format!("{id}:{:?}", event.source))
                .or_default() += 1;
        }
        if event.kind == CombatLogKind::CastSuccess && event.source == Some(actor.to_bits()) {
            *casts.entry(id).or_default() += 1;
        }
        if event.kind == CombatLogKind::Energize && event.target == Some(actor.to_bits()) {
            *energize.entry(id).or_default() += i64::from(event.amount);
        }
        if matches!(
            event.kind,
            CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
        ) && event.source == Some(actor.to_bits())
        {
            *aura_applications.entry(id).or_default() += 1;
        }
    }
    assert!(
        app.world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current
            > 0.0,
        "follower died before completing the dummy measurement"
    );
    if entry == 209053 && level == 90 && std::env::var_os("FOLLOWER_DPS_WORLD_DB").is_some() {
        assert!(
            !known_spells.contains(&117014),
            "Shuja's profile does not select Elemental Blast"
        );
        assert!(
            hits.get(&117014).is_some_and(|count| *count > 0),
            "Fusion must produce actual Elemental Blast hits"
        );
        assert!(
            [118522, 173183, 173184]
                .iter()
                .any(|id| aura_applications.get(id).is_some_and(|count| *count > 0)),
            "Fusion Blast must grant an Elemental Blast stat buff without the separate talent"
        );
        assert!(
            storm_frames > 0,
            "Storm Elemental must actually be summoned"
        );
        assert_eq!(
            lava_during_storm, 0,
            "Shuja must hold Lava Burst during her non-Primal Storm Elemental"
        );
        assert_eq!(
            lava_during_stable_storm, 0,
            "instant Lava Surge casts must also wait during Storm Elemental"
        );
        assert!(
            bolt_during_storm > 0,
            "Shuja must use Lightning Bolt during Storm Elemental"
        );
        assert!(
            casts.get(&51505).is_some_and(|count| *count > 0),
            "Lava Burst resumes outside Storm Elemental"
        );
    }
    if entry == 209057 && std::env::var_os("FOLLOWER_VERIFY_WINGS_ORDER").is_some() {
        let wings: Vec<f64> = cast_trace
            .iter()
            .filter(|action| action["spell"] == 389539)
            .map(|action| action["at_s"].as_f64().unwrap())
            .collect();
        let tolls: Vec<f64> = cast_trace
            .iter()
            .filter(|action| action["spell"] == 375576)
            .map(|action| action["at_s"].as_f64().unwrap())
            .collect();
        assert_eq!(
            (wings.len(), tolls.len()),
            (2, 2),
            "both cooldown windows must occur"
        );
        for (&wing, &toll) in wings.iter().zip(&tolls) {
            assert!(
                wing < toll && toll - wing < 2.0,
                "Sentinel {wing}s must precede Toll {toll}s"
            );
        }
    }
    if entry == 209057 && std::env::var_os("FOLLOWER_DPS_WORLD_DB").is_some() {
        assert!(
            !hits.contains_key(&1269224),
            "Blaze of Glory must not hit Shield of the Righteous's primary target"
        );
        assert!(
            casts.get(&389539).is_some_and(|count| *count > 0),
            "Sentinel must start"
        );
        assert!(
            casts.get(&1241413).is_some_and(|count| *count > 0),
            "Sentinel must enable the actual Hammer of Wrath action"
        );
        assert!(
            damage.get(&1241413).is_some_and(|amount| *amount > 0),
            "Hammer of Wrath must damage the dummy"
        );
    }
    if entry == 209053 && std::env::var_os("FOLLOWER_DPS_WORLD_DB").is_some() {
        assert!(known_spells.contains(&378255) && known_spells.contains(&191717));
        assert!(!known_spells.contains(&117013));
        assert!(
            damage.get(&57984).is_some_and(|amount| *amount > 0),
            "Greater Fire must damage the dummy"
        );
        assert!(
            damage.get(&157331).is_some_and(|amount| *amount > 0),
            "Greater Storm must damage the dummy"
        );
    }
    if entry == 209065 {
        assert!(
            incoming.iter().all(|event| event.spell_id != Some(321538)),
            "owned pets must not apply Bloodshed to the follower"
        );
        assert!(
            damage.get(&321538).is_some_and(|amount| *amount > 0),
            "owned pets must damage the selected enemy with Bloodshed"
        );
    }
    if let Some(capture) = app
        .world()
        .get_resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
    {
        println!("FOLLOWER_FACTORS {}", capture.export());
    }
    println!(
        "FOLLOWER_DPS {}",
        json!({"entry":entry,"level":level,"map":map,"difficulty":205,"seed":seed,"seconds":120,"sheet":sheet,
        "items":items,"modifiers":modifiers,"talents":talents,"known_spells":known_spells,"target_armor":armor,"checkpoints":checkpoints,"incoming":incoming,"actor_id":actor.to_bits(),"target_id":target.to_bits(),"damage":damage,"hits":hits,"hit_breakdown":hit_breakdown,
        "mean_stats":stat_sums.map(|sum| sum / 2400.0),"sources":damage_sources,"casts":casts,"cast_trace":cast_trace,"aura_applications":aura_applications,"energize":energize,
        "flame_shock_s":flame_shock_frames as f64 / 20.0,"lava_surge_s":lava_surge_frames as f64 / 20.0,
        "storm_elemental_s":storm_frames as f64 / 20.0,"lava_casts_started_during_storm":lava_during_storm,
        "bolt_casts_started_during_storm":bolt_during_storm,"lava_successes_during_stable_storm":lava_during_stable_storm,"casting_s":casting as f64 / 20.0,
        "gcd_s":gcd_total as f64 / 20.0,"gcd_only_s":gcd_only as f64 / 20.0,"ready_s":ready as f64 / 20.0})
    );
}
