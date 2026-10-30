//! Production follower gear and AI, measured independently of parity drivers.
use super::*;
use shared::components::{Level, UnitLevel, UnitSummonedBy};
use shared::protocol::CombatLogKind;

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
    let mut app = super::super::runtime::build_app();
    if let Some(path) = std::env::var_os("FOLLOWER_DPS_WORLD_DB") {
        let data = crate::world_data::load_world_data(std::path::Path::new(&path));
        app.insert_resource(data.armor_constants.clone());
        app.insert_resource(data);
        let db =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        app.insert_resource(crate::followers::simulator::load_parity_catalog(&db).unwrap());
    }
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    app.insert_resource(crate::item_proc_data::load(&db).unwrap());
    crate::spell_cast::area_trigger::install_native_catalog(app.world_mut(), &db).unwrap();
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
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
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
    let mut casting = 0u32;
    let mut gcd_only = 0u32;
    let mut gcd_total = 0u32;
    let mut ready = 0u32;
    let mut sources = std::collections::BTreeSet::from([actor.to_bits()]);
    let mut checkpoints = Vec::new();
    for frame in 0..2400 {
        // SimC fixed-time Patchwerk uses a 100% to 0% target-health timeline.
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * (2400 - frame) as f32 / 2400.0;
        app.update();
        let world = app.world();
        let now = crate::spell_cast::history_now(world);
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
    let mut casts = BTreeMap::<u32, usize>::new();
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
        }
        if event.kind == CombatLogKind::CastSuccess && event.source == Some(actor.to_bits()) {
            *casts.entry(id).or_default() += 1;
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
    if entry == 209057 && std::env::var_os("FOLLOWER_DPS_WORLD_DB").is_some() {
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
    println!(
        "FOLLOWER_DPS {}",
        json!({"entry":entry,"level":level,"map":map,"difficulty":205,"seed":seed,"seconds":120,"sheet":sheet,
        "items":items,"modifiers":modifiers,"talents":talents,"known_spells":known_spells,"target_armor":armor,"checkpoints":checkpoints,"incoming":incoming,"actor_id":actor.to_bits(),"target_id":target.to_bits(),"damage":damage,"hits":hits,"casts":casts,"casting_s":casting as f64 / 20.0,
        "gcd_s":gcd_total as f64 / 20.0,"gcd_only_s":gcd_only as f64 / 20.0,"ready_s":ready as f64 / 20.0})
    );
}
