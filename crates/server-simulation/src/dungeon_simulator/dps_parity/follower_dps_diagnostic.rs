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
    let _seed = crate::random::seed_scope(1);
    let mut app = super::super::runtime::build_app();
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
    app.world_mut()
        .entity_mut(target)
        .insert((Level(level), UnitLevel(level)));
    app.world_mut()
        .entity_mut(target)
        .remove::<shared::level_scaling::LevelScaling>();
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    app.world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(target)));
    app.world_mut()
        .entity_mut(owner)
        .insert(crate::combat::AutoAttack::new(target));
    crate::combat_refs::set_in_combat_with(app.world_mut(), owner, target);
    for _ in 0..5 {
        app.update();
    }
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
    let mut ready = 0u32;
    let mut sources = std::collections::BTreeSet::from([actor.to_bits()]);
    for _ in 0..2400 {
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
        if world.get::<shared::casting::CastState>(actor).is_some() {
            casting += 1;
        } else if gcd {
            gcd_only += 1;
        } else {
            ready += 1;
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
    let mut casts = BTreeMap::<u32, usize>::new();
    for event in events {
        if !event.source.is_some_and(|id| sources.contains(&id)) {
            continue;
        }
        let id = event.spell_id.unwrap_or(0);
        if event.kind == CombatLogKind::Damage && event.target == Some(target.to_bits()) {
            *damage.entry(id).or_default() += i64::from(event.amount);
        }
        if event.kind == CombatLogKind::CastSuccess && event.source == Some(actor.to_bits()) {
            *casts.entry(id).or_default() += 1;
        }
    }
    println!(
        "FOLLOWER_DPS {}",
        json!({"entry":entry,"level":level,"map":map,"difficulty":205,"seconds":120,"sheet":sheet,
        "items":items,"modifiers":modifiers,"damage":damage,"casts":casts,"casting_s":casting as f64 / 20.0,
        "gcd_only_s":gcd_only as f64 / 20.0,"ready_s":ready as f64 / 20.0})
    );
}
