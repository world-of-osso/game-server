//! Production follower gear and AI, measured independently of parity drivers.
use super::*;
use shared::components::UnitSummonedBy;
use shared::protocol::CombatLogKind;

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
    let app = build_snapshot_app(&path).unwrap();
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

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with ItemSpec data"]
fn follower_vengeance_snapshot_equips_spec_eligible_weapon() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (app, actor, _, _) = prepare_class_follower(app, copy, 581, 90);
    let items = &app
        .world()
        .get::<crate::stats::EquippedItems>(actor)
        .unwrap()
        .items;
    let main = items
        .iter()
        .find(|(slot, _, _)| *slot == shared::protocol::EquipmentSlot::MainHand)
        .unwrap();
    assert_ne!(main.2, 251_128);
    let template = app
        .world()
        .resource::<crate::item_templates::ItemTemplates>()
        .get(main.1, main.2)
        .unwrap();
    assert!(matches!(template.scaling.subclass_id, 0 | 7 | 9 | 13));
}

fn prepare_follower(
    entry: u32,
    level: u8,
    map: u32,
) -> (App, Entity, Entity, crate::followers::Profile) {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB")
        .expect("FOLLOWER_DPS_WORLD_DB must name the measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, map).unwrap();
    let profile = app
        .world()
        .resource::<crate::followers::FollowerCatalog>()
        .profiles
        .iter()
        .find(|profile| profile.entry == entry)
        .unwrap()
        .clone();
    prepare_class_follower(app, copy, profile.spec_id, level)
}

pub(super) fn prepare_class_follower(
    mut app: App,
    copy: crate::maps::InMap,
    spec: u32,
    level: u8,
) -> (App, Entity, Entity, crate::followers::Profile) {
    let actor = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        spec,
        level,
        copy,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    let profile = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .profile
        .clone();
    let target = spawn_level_dummy(&mut app, actor, level, 0).unwrap();
    (app, actor, target, profile)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with level-specific Ret profiles"]
fn follower_ret_radiant_glory_wake_activates_wrath_only_when_selected() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    for (level, map, selected) in [(70, 2515, true), (90, 2813, false)] {
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, map).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 70, level);
        let traits = app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap();
        assert_eq!(
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == 458359),
            selected
        );
        assert!(!crate::auras::has_aura(app.world(), actor, 454351));
        crate::spell_cast::creature_cast(app.world_mut(), actor, 255937, Some(target), false)
            .unwrap();
        assert_eq!(crate::auras::has_aura(app.world(), actor, 454351), selected);
        assert_eq!(
            crate::auras::has_aura(app.world(), actor, 1277026),
            selected
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Ret follower profiles"]
fn follower_ret_hammer_selected_talent_admits_real_wrath_cast() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    for (level, map) in [(70, 2515), (90, 2813)] {
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, map).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 70, level);
        let known = app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap();
        assert!(known.contains(1241288));
        assert!(!known.contains(24275));
        assert_eq!(
            crate::spell_cast::knowledge::resolve_action(app.world(), actor, 24275).unwrap(),
            Some(24275)
        );
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 31884, Some(actor), 0);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 24275, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            app.update();
        }
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(24275)
                    && event.amount > 0)
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_execute_applies_one_deep_wounds_deposit() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 70);
    for _ in 0..5 {
        app.update();
    }
    let mut health = app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap();
    health.current = health.max * 0.1;
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::Rage)
        .unwrap()
        .current = 400.0;
    app.init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 281000, Some(target), false).unwrap();
    for _ in 0..20 {
        app.update();
    }
    let casts = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0;
    let deposits = casts
        .iter()
        .filter(|(caster, spell, _)| *caster == actor && *spell == 262115)
        .count();
    assert_eq!(
        deposits, 1,
        "one Execute impact must deposit Deep Wounds once: {casts:?}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_doomguard_casts_native_volley_during_its_lifetime() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::networking::CombatTarget(Some(target)))
        .remove::<crate::followers::Follower>();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 1276672, Some(target), false).unwrap();
    for _ in 0..40 {
        app.update();
    }
    let pet = {
        let mut query = app.world_mut().query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>();
        query
            .iter(app.world())
            .find(|(_, summon, npc)| {
                summon.owner == actor && summon.spell_id == 1276672 && npc.template_id == 250785
            })
            .map(|(entity, _, _)| entity)
            .expect("real selected Doomguard must be summoned")
    };
    for _ in 0..160 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let volleys = events
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.spell_id == Some(1251989)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        volleys >= 2,
        "actual Doomguard must repeatedly land native volleys: {volleys}"
    );
    for _ in 0..80 {
        app.update();
    }
    assert!(
        app.world().get_entity(pet).is_err(),
        "native Doomguard expires after its authored lifetime"
    );
}

fn prepare_marksmanship_70() -> (App, Entity, Entity) {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 254, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    (app, actor, target)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mm_steady_shot_reduces_running_aimed_recharge() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    for trueshot in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 254, 90);
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        crate::spell_cast::creature_cast(app.world_mut(), actor, 19434, Some(target), false)
            .unwrap();
        for _ in 0..70 {
            app.update();
        }
        if trueshot {
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 288613, Some(actor), 1);
            for _ in 0..2 {
                app.update();
            }
        }
        let bindings = Bindings {
            actions: [("aimed_shot".into(), 19434)].into(),
            ..default()
        };
        let encounter = Encounter::Fixture {
            started_at: 0.0,
            duration: 120.0,
            style: FightStyle::Patchwerk,
        };
        let recharge = |app: &App| {
            NativeFacts::new(app.world(), actor, &bindings, encounter, 1)
                .value("cooldown.aimed_shot.full_recharge_time")
                .unwrap()
        };
        let before = recharge(&app);
        let start = crate::spell_cast::history_now(app.world());
        crate::spell_cast::creature_cast(app.world_mut(), actor, 56641, Some(target), false)
            .unwrap();
        for _ in 0..55 {
            app.update();
        }
        let elapsed = crate::spell_cast::history_now(app.world()) - start;
        let reduction = before - elapsed - recharge(&app);
        let expected = if trueshot { 2.0 / 1.4 } else { 2.0 };
        assert!(
            (reduction - expected).abs() < 0.1,
            "Steady Shot recharge reduction={reduction}, expected={expected}, Trueshot={trueshot}"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mm_take_aim_mark_is_earned_and_consumed_by_rapid_fire() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 254, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 288613, Some(target), false).unwrap();
    for _ in 0..10 {
        app.update();
    }
    assert!(
        crate::auras::has_aura(app.world(), target, 1301098),
        "real Trueshot earns Take Aim mark"
    );
    for _ in 0..40 {
        app.update();
    }
    crate::spell_cast::creature_cast(app.world_mut(), actor, 257044, Some(target), false).unwrap();
    for _ in 0..60 {
        app.update();
    }
    assert!(
        !crate::auras::has_aura(app.world(), target, 1301098),
        "Rapid Fire consumes earned mark"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mm_take_aim_new_mark_survives_in_flight_rapid_tick() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 254, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 257044, Some(target), false).unwrap();
    let mut channel_started = false;
    for _ in 0..100 {
        app.update();
        let active = crate::auras::has_aura(app.world(), target, 257044);
        channel_started |= active;
        if channel_started && !active {
            break;
        }
    }
    assert!(
        channel_started && !crate::auras::has_aura(app.world(), target, 257044),
        "Rapid Fire channel must start and finish"
    );
    let count = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| event.spell_id == Some(257045) && event.kind == CombatLogKind::Damage)
            .count()
    };
    let before = count(&app);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 1301098, Some(target), 1);
    assert!(crate::auras::has_aura(app.world(), target, 1301098));
    for _ in 0..20 {
        app.update();
    }
    assert!(
        count(&app) > before,
        "a Rapid Fire missile actually landed after the channel"
    );
    assert!(
        crate::auras::has_aura(app.world(), target, 1301098),
        "a previously launched missile must not consume the new mark"
    );
}

fn take_aim_damage_receipt(marked: bool, spell: u32) -> (f64, Vec<i32>) {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 254, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    if marked {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 1301098, Some(target), 1);
    }
    let observed_spell = if spell == 257044 { 257045 } else { spell };
    app.world_mut().insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            actor,
            target,
            observed_spell,
            "take_aim",
            20,
        ),
    );
    crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(target), false).unwrap();
    for _ in 0..90 {
        app.update();
    }
    let receipt = app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let first = receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == observed_spell)
        .expect("actual landed damage");
    let crit = first["factors"]["crit"].as_f64().unwrap();
    let damage = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(observed_spell) && event.kind == CombatLogKind::Damage
        })
        .map(|event| event.amount)
        .collect();
    (crit, damage)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mm_take_aim_mark_increases_actual_aimed_crit_and_all_rapid_ticks() {
    let (plain_crit, _) = take_aim_damage_receipt(false, 19434);
    let (marked_crit, _) = take_aim_damage_receipt(true, 19434);
    assert!(
        (marked_crit - plain_crit - 0.5).abs() < 0.001,
        "actual Aimed crit {plain_crit} -> {marked_crit}"
    );
    let (_, plain) = take_aim_damage_receipt(false, 257044);
    let (_, marked) = take_aim_damage_receipt(true, 257044);
    assert!(
        plain.len() >= 3 && plain.len() == marked.len(),
        "Rapid Fire ticks {plain:?} / {marked:?}"
    );
    // Applying the mark advances the random stream, so weapon variance differs
    // between the two runs. Check every tick and the aggregate separately.
    for (a, b) in plain.iter().zip(&marked) {
        assert!(
            (*b as f64 / *a as f64 - 1.55).abs() < 0.08,
            "each actual Rapid tick must retain the consumed mark: {plain:?} / {marked:?}"
        );
    }
    let ratio = marked.iter().sum::<i32>() as f64 / plain.iter().sum::<i32>() as f64;
    assert!(
        (ratio - 1.55).abs() < 0.04,
        "actual channel damage ratio {ratio}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mm_lock_and_load_explosive_restores_one_aimed_charge() {
    let _seed = crate::random::seed_scope(70448);
    let (mut app, actor, target) = prepare_marksmanship_70();
    for _ in 0..2 {
        crate::spell_cast::creature_cast(app.world_mut(), actor, 19434, Some(target), false)
            .unwrap();
        for _ in 0..70 {
            app.update();
        }
    }
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("aimed_shot".into(), 19434)].into(),
        ..default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: 0.0,
        duration: 120.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let charges = |app: &App| {
        let facts =
            crate::simc_apl_native::NativeFacts::new(app.world(), actor, &bindings, encounter, 1);
        server::simc_apl::Facts::value(&facts, "cooldown.aimed_shot.charges").unwrap()
    };
    assert_eq!(charges(&app), 0.0, "actual Aimed casts spend both charges");
    crate::spell_cast::creature_cast(app.world_mut(), actor, 212431, Some(target), false).unwrap();
    for _ in 0..4 {
        app.update();
    }
    assert!(
        crate::auras::has_aura(app.world(), actor, 194594),
        "selected Tactical Reload earns Lock and Load"
    );
    assert_eq!(
        charges(&app),
        1.0,
        "new Lock and Load restores one Aimed charge"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 194594, Some(actor), 1);
    assert_eq!(
        charges(&app),
        1.0,
        "refreshing the existing buff cannot restore a second charge"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mm_lock_and_load_rapid_fire_earns_windrunner_proc() {
    let _seed = crate::random::seed_scope(70448);
    let (mut app, actor, target) = prepare_marksmanship_70();
    for _ in 0..16 {
        crate::spell_cast::creature_cast(app.world_mut(), actor, 257044, Some(target), false)
            .unwrap();
        for _ in 0..55 {
            app.update();
        }
        if crate::auras::has_aura(app.world(), actor, 194594) {
            return;
        }
        crate::spell_cast::scripts::reduce_cooldown(app.world_mut(), actor, 257044, 120.0);
    }
    panic!("selected Windrunner Quiver must earn Lock and Load on a real Rapid Fire");
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mm_lock_and_load_salvo_earns_tactical_reload() {
    let _seed = crate::random::seed_scope(70448);
    let (mut app, actor, target) = prepare_marksmanship_70();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 260243, Some(target), false).unwrap();
    for _ in 0..80 {
        app.update();
    }
    assert!(
        crate::auras::has_aura(app.world(), actor, 194594),
        "an actual Salvo pulse earns Tactical Reload"
    );
}

fn flame_shock_mastery_sample(keep_mastery: bool) -> (f32, i32, bool, i32, bool) {
    let _seed = crate::random::seed_scope(1);
    let (mut app, actor, target, _) = prepare_follower(209053, 90, 2813);
    for _ in 0..5 {
        app.update();
    }
    let mastery = app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 168534 && aura.caster == actor)
        .expect("Elemental follower mastery aura")
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("native direct mastery modifier")
        .amount;
    if !keep_mastery {
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 168534 && aura.caster == actor,
        );
    }
    app.world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    assert!(crate::spell_cast::try_cast_triggered(
        app.world_mut(),
        actor,
        188389,
        Some(target),
        1
    ));
    for _ in 0..80 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hit = |periodic| {
        events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(188389)
                    && event.periodic == periodic
            })
            .expect("actual Flame Shock damage")
    };
    let direct = hit(false);
    let periodic = hit(true);
    (
        mastery,
        direct.amount,
        direct.crit,
        periodic.amount,
        periodic.crit,
    )
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_elemental_flame_shock_uses_mastery_on_direct_and_periodic_damage() {
    let (mastery, direct, direct_crit, periodic, periodic_crit) = flame_shock_mastery_sample(true);
    let (_, plain_direct, plain_direct_crit, plain_periodic, plain_periodic_crit) =
        flame_shock_mastery_sample(false);
    assert_eq!(
        (direct_crit, periodic_crit),
        (plain_direct_crit, plain_periodic_crit)
    );
    let expected = 1.0 + f64::from(mastery) / 100.0;
    let direct_ratio = direct as f64 / plain_direct as f64;
    let periodic_ratio = periodic as f64 / plain_periodic as f64;
    assert!(
        (direct_ratio - expected).abs() < 0.02,
        "direct {direct_ratio}, expected {expected}"
    );
    assert!(
        (periodic_ratio - expected).abs() < 0.02,
        "periodic {periodic_ratio}, expected {expected}"
    );
}

/// The production dummy's 120 s SimC `max_time` window in 50 ms frames.
pub(super) const FIXED_FIGHT_FRAMES: u32 = 2400;

/// Engage the dummy for a SimC fixed-time fight of `seconds`. SimC76f8118
/// sim.cpp3582–3585: `fight_remains` is the iteration length minus elapsed time,
/// not the dummy's projected time to die.
pub(super) fn start_fixed_time_fight(
    app: &mut App,
    owner: Entity,
    actor: Entity,
    target: Entity,
    seconds: f64,
) {
    app.insert_resource(server_follower_apl::registry::FixedLengthFight { duration: seconds });
    app.world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(target)));
    app.world_mut()
        .entity_mut(owner)
        .insert(crate::combat::AutoAttack::new(target));
    crate::combat_refs::set_in_combat_with(app.world_mut(), owner, target);
    // The default list first runs in the tick precombat finishes; restart the
    // dummy's lifetime with the fight clock afterwards.
    install_fixed_fight_dummy(app, target, seconds);
    finish_follower_precombat(app, actor);
    install_fixed_fight_dummy(app, target, seconds);
}

/// SimC76f8118 player.cpp7743–7754: with fixed_time the dummy's time to die is
/// the iteration length minus elapsed time; player.hpp964: a tank dummy is a boss.
fn install_fixed_fight_dummy(app: &mut App, target: Entity, seconds: f64) {
    let lifetime = std::time::Duration::from_secs_f64(seconds);
    let expires_at = app.world().resource::<Time>().elapsed() + lifetime;
    app.world_mut()
        .entity_mut(target)
        .insert(crate::simc_apl_native::FixtureTarget {
            expires_at,
            lifetime,
            is_boss: true,
        });
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn blood_follower_spends_runic_power_inside_a_fixed_fights_last_ten_seconds() {
    // SimC76f8118 apl_death_knight.cpp204:
    // death_strike,if=runic_power.deficit<40|fight_remains<10|...
    let death_strikes = |seconds: f64| {
        let _seed = crate::random::seed_scope(1);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 250, 90);
        for _ in 0..5 {
            app.update();
        }
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        start_fixed_time_fight(&mut app, owner, actor, target, seconds);
        for _ in 0..200 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        // Hold 60 Runic Power (deficit 65) so only fight_remains can select Death Strike
        // over the following heart_strike,if=rune>=2 / blood_boil lines.
        for _ in 0..160 {
            let power = shared::components::PowerType::RunicPower;
            let current = crate::spell_cast::powers::power_value(app.world(), actor, power)
                .unwrap()
                .0;
            crate::spell_cast::powers::change_power(app.world_mut(), actor, power, 600.0 - current);
            app.update();
        }
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(actor.to_bits())
                    && event.spell_id == Some(49998)
            })
            .count()
    };
    let ending = death_strikes(20.0);
    let long = death_strikes(300.0);
    assert!(
        ending > long,
        "a fight ending within 10 s spends Runic Power on Death Strike: {ending} vs {long}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn ttd_reading_followers_keep_casting_against_the_fixed_fight_dummy() {
    // Vengeance reads target.time_to_die for dung_pull_ttd every pass; Arms
    // for its execute timing. Both failed without the dummy's lifetime.
    for (spec, level, map) in [(581, 70, 2515), (71, 90, 2813)] {
        let _seed = crate::random::seed_scope(1);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, map).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, spec, level);
        for _ in 0..5 {
            app.update();
        }
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
        let started = app.world().resource::<Time>().elapsed();
        for _ in 0..200 {
            app.update();
        }
        assert!(
            app.world()
                .get::<server_follower_apl::FollowerAplFailed>(actor)
                .is_none(),
            "spec{spec} APL failed in the fixed fight"
        );
        let elapsed = (app.world().resource::<Time>().elapsed() - started).as_secs_f64();
        let remaining = app
            .world()
            .get::<crate::simc_apl_native::FixtureTarget>(target)
            .unwrap()
            .remaining(app.world().resource::<Time>().elapsed())
            .as_secs_f64();
        assert!(
            (remaining - (120.0 - elapsed)).abs() < 1e-6,
            "spec{spec}: {remaining}"
        );
        let casts = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess && event.source == Some(actor.to_bits())
            })
            .count();
        assert!(casts > 0, "spec{spec} cast nothing in 10 s");
    }
}

/// SimC runs precombat before the fixed-time target-health timeline starts.
fn finish_follower_precombat(app: &mut App, actor: Entity) {
    for _ in 0..800 {
        let precombat_cast = app
            .world()
            .get::<shared::casting::CastState>(actor)
            .map(|cast| cast.spell_id);
        app.world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        app.update();
        assert!(
            app.world()
                .get::<server_follower_apl::FollowerAplFailed>(actor)
                .is_none(),
            "follower APL failed during precombat"
        );
        if app
            .world()
            .get::<server_follower_apl::FollowerApl>(actor)
            .is_some_and(|apl| apl.in_combat())
        {
            let mut events = app
                .world_mut()
                .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>();
            if let Some(spell) = precombat_cast
                && let Some(index) = events.0.iter().position(|event| {
                    event.source == Some(actor.to_bits())
                        && event.kind == CombatLogKind::CastSuccess
                        && event.spell_id == Some(spell)
                })
            {
                events.0.remove(index);
            }
            return;
        }
    }
    panic!("follower precombat did not reach combat");
}

fn class_follower_combat_fixture(spec: u32) -> (App, Entity, Entity) {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, enemy, _) = prepare_class_follower(app, copy, spec, 90);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    app.world_mut()
        .entity_mut(owner)
        .insert(shared::components::Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    app.world_mut().entity_mut(actor).insert((
        shared::components::Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        shared::components::Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    app.world_mut()
        .entity_mut(enemy)
        .insert(shared::components::Position {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .entity_mut(actor)
        .insert(shared::components::Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        });
    (app, actor, enemy)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_combat_player_like_tank_uses_defense_against_enemy_autoattacks() {
    use shared::protocol::MissKind;
    let _seed = crate::random::seed_scope(1);
    let (mut app, actor, enemy) = class_follower_combat_fixture(66);
    app.world_mut()
        .entity_mut(enemy)
        .insert(crate::combat::AutoAttack::new(actor));
    crate::combat_refs::set_in_combat_with(app.world_mut(), enemy, actor);
    for kind in 0..3 {
        {
            let mut stats = app
                .world_mut()
                .get_mut::<crate::stats::CharacterStats>(actor)
                .unwrap();
            stats.defense.dodge = if kind == 0 { 100.0 } else { 0.0 };
            stats.defense.parry = if kind == 1 { 100.0 } else { 0.0 };
            stats.defense.block = if kind == 2 { 100.0 } else { 0.0 };
            stats.defense.shield_block = 10000.0;
        }
        let start = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        for _ in 0..400 {
            app.update();
        }
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..];
        let defended = events
            .iter()
            .filter(|event| {
                event.source == Some(enemy.to_bits()) && event.target == Some(actor.to_bits())
            })
            .any(|event| match kind {
                0 => event.kind == CombatLogKind::Miss(MissKind::Dodge),
                1 => event.kind == CombatLogKind::Miss(MissKind::Parry),
                _ => event.blocked > 0,
            });
        assert!(defended, "class tank did not use defense case {kind}");
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_combat_player_like_melee_uses_player_white_miss_table() {
    use shared::protocol::MissKind;
    let _seed = crate::random::seed_scope(1);
    let (mut app, actor, enemy) = class_follower_combat_fixture(250);
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::combat::AutoAttack::new(enemy));
    crate::combat_refs::set_in_combat_with(app.world_mut(), actor, enemy);
    for _ in 0..1000 {
        if let Some(mut attack) = app.world_mut().get_mut::<crate::combat::AutoAttack>(actor) {
            attack.timer = std::time::Duration::ZERO;
        }
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let swings: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(enemy.to_bits())
                && matches!(event.spell_id, None | Some(0))
        })
        .collect();
    assert!(
        swings.len() >= 100,
        "expected sustained native autoattacks, got {}",
        swings.len()
    );
    assert!(
        !swings
            .iter()
            .any(|event| event.kind == CombatLogKind::Miss(MissKind::Miss)),
        "same-level player white swings cannot miss"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_combat_player_like_guardian_form_uses_player_attack_timing() {
    let (mut app, actor, _) = class_follower_combat_fixture(104);
    assert!(crate::spell_cast::try_cast_triggered(
        app.world_mut(),
        actor,
        5487,
        Some(actor),
        1
    ));
    let bear = crate::auras::shapeshift::attack_timing(app.world(), actor)
        .expect("native Bear Form timing");
    assert_eq!(bear.seconds, 2.5);
    assert!(bear.use_form_ap_speed);
    assert!(crate::spell_cast::try_cast_triggered(
        app.world_mut(),
        actor,
        426268,
        Some(actor),
        1
    ));
    let nonferal = crate::auras::shapeshift::attack_timing(app.world(), actor)
        .expect("native Chi-Ji form timing");
    assert_eq!(nonferal.seconds, 2.0);
    assert!(
        !nonferal.use_form_ap_speed,
        "class actor uses the player nonferal AP multiplier"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn class_blood_follower_regenerates_runes_and_keeps_casting() {
    let _seed = crate::random::seed_scope(1);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 250, 90);
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
    finish_follower_precombat(&mut app, actor);
    for _ in 0..600 {
        app.update();
    }
    let start = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..600 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let casts = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(195292 | 195182 | 206930 | 433895))
        })
        .count();
    assert!(
        casts > 0,
        "the follower must recharge runes and cast after 30 seconds"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn blood_follower_death_strike_counts_damage_beyond_current_health() {
    use server::test_support::healer_intake::{Hit, HitKind};
    use shared::components::Health;

    fn strike(start_health_fraction: f32) -> (i32, i32, i32, f32) {
        let _seed = crate::random::seed_scope(1);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 250, 90);
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
        finish_follower_precombat(&mut app, actor);
        app.world_mut()
            .entity_mut(actor)
            .insert(server::test_support::SimcInfiniteHealth);
        let max = app.world().get::<Health>(actor).unwrap().max;
        app.world_mut().get_mut::<Health>(actor).unwrap().current = max * start_health_fraction;
        let hit = Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: max * 0.9,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: "Hit".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        let start = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::combat::replay_healer_hit(app.world_mut(), target, actor, &hit).unwrap();
        for _ in 0..4 {
            app.update();
        }
        let damage = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..]
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Damage
                    && event.target == Some(actor.to_bits())
                    && event.source == Some(target.to_bits())
            })
            .unwrap();
        let (amount, overflow) = (damage.amount, damage.overflow);
        app.world_mut().get_mut::<Health>(actor).unwrap().current = max * 0.01;
        assert!(crate::spell_cast::try_cast_triggered(
            app.world_mut(),
            actor,
            49998,
            Some(target),
            1
        ));
        for _ in 0..4 {
            app.update();
        }
        let heal = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..]
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.source == Some(actor.to_bits())
                    && event.spell_id == Some(45470)
            })
            .expect("Death Strike heal")
            .amount;
        (amount, overflow, heal, max)
    }

    let (full_damage, full_overflow, full_heal, max) = strike(1.0);
    let (low_damage, low_overflow, low_heal, _) = strike(0.01);
    assert_eq!(full_damage, low_damage);
    assert_eq!(full_overflow, 0);
    assert!(low_overflow > 0, "low-health hit must report overkill");
    assert!(
        full_heal as f32 > max * 0.1,
        "damage-based heal must exceed the minimum"
    );
    assert_eq!(
        low_heal, full_heal,
        "remaining health must not reduce the recorded damage"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn blood_draw_triggers_only_once_during_its_cooldown() {
    use server::test_support::healer_intake::{Hit, HitKind};
    use shared::components::Health;
    use std::time::Duration;

    let _seed = crate::random::seed_scope(1);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 250, 70);
    for _ in 0..5 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 374598));
    app.world_mut()
        .entity_mut(actor)
        .insert(server::test_support::SimcInfiniteHealth);
    let max = app.world().get::<Health>(actor).unwrap().max;
    let hit = Hit {
        tick: 0,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: max * 0.1,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    let hit_below_threshold = |app: &mut App| {
        app.world_mut().get_mut::<Health>(actor).unwrap().current = max * 0.2;
        crate::combat::replay_healer_hit(app.world_mut(), target, actor, &hit).unwrap();
        for _ in 0..4 {
            app.update();
        }
    };
    let reduction_applications = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.target == Some(actor.to_bits())
                    && event.spell_id == Some(454871)
                    && matches!(
                        event.kind,
                        CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                    )
            })
            .count()
    };
    hit_below_threshold(&mut app);
    assert!(crate::auras::has_aura(app.world(), actor, 374609));
    assert!(crate::auras::has_aura(app.world(), actor, 454871));
    assert_eq!(reduction_applications(&app), 1);
    hit_below_threshold(&mut app);
    assert_eq!(
        reduction_applications(&app),
        1,
        "cooldown must block another Blood Draw"
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(250),
    ));
    for _ in 0..484 {
        app.update();
    }
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    assert!(!crate::auras::has_aura(app.world(), actor, 374609));
    assert!(!crate::auras::has_aura(app.world(), actor, 454871));
    hit_below_threshold(&mut app);
    assert_eq!(
        reduction_applications(&app),
        2,
        "Blood Draw must become available after its cooldown"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn form_override_guardian_swipe_and_form_switch() {
    let _seed = crate::random::seed_scope(1);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 104, 70);
    for _ in 0..5 {
        app.update();
    }
    let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE").expect("Guardian pressure trace");
    let mut scenario = super::follower_tank_scenario::TankScenario::read(
        std::path::Path::new(&trace),
        app.world_mut(),
        target,
        actor,
        70,
    )
    .unwrap();
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
    finish_follower_precombat(&mut app, actor);
    for _ in 0..60 {
        app.update();
        if crate::auras::has_aura(app.world(), actor, 5487) {
            break;
        }
    }
    assert!(
        app.world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(213764)
    );
    assert!(crate::auras::has_aura(app.world(), actor, 106829));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 213764).unwrap(),
        213771
    );
    for tick in 0..1000 {
        scenario.advance(app.world_mut(), tick * 50).unwrap();
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(213771)
                && event.kind == CombatLogKind::CastSuccess),
        "Guardian APL must cast Bear Swipe"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(213771)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(target.to_bits())
                && event.amount > 0),
        "Guardian APL must land Bear Swipe; actor events: {:?}; APL failure: {:?}",
        events
            .iter()
            .filter(|event| event.source == Some(actor.to_bits())
                && matches!(
                    event.kind,
                    CombatLogKind::CastSuccess | CombatLogKind::Damage
                ))
            .map(|event| (event.spell_id, event.kind, event.target, event.amount))
            .collect::<Vec<_>>(),
        app.world()
            .get::<server_follower_apl::FollowerAplFailed>(actor)
            .is_some()
    );
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 768,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(!crate::auras::has_aura(app.world(), actor, 106829));
    assert!(crate::auras::has_aura(app.world(), actor, 48629));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 213764).unwrap(),
        106785
    );
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 213771).is_err()
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn form_override_warrior_aura_grants_and_revokes_replacement() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, _, _) = prepare_class_follower(app, copy, 72, 70);
    app.world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(actor)
        .unwrap()
        .grant_fixture_spell(23881);
    for _ in 0..5 {
        app.update();
    }
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 335096).is_err()
    );
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 1719,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 23881).unwrap(),
        335096
    );
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 335096).unwrap(),
        335096
    );
    crate::auras::remove_auras(
        app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1719 && aura.caster == actor,
    );
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 335096).is_err()
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn form_override_player_presets_end_with_form() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let actor = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        250,
        70,
        copy,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    for _ in 0..5 {
        app.update();
    }
    assert!(
        app.world()
            .get::<shared::components::Player>(actor)
            .is_some()
    );
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    assert!(
        !app.world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(50799)
    );
    assert!(crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 50799).is_err());
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 52509,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 50799).unwrap(),
        50799
    );
    assert!(
        !app.world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(50799)
    );
    crate::auras::remove_auras(
        app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 52509 && aura.caster == actor,
    );
    assert!(crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 50799).is_err());
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_dummy_starts_measurement_after_precombat() {
    let (mut app, actor, target, _) = prepare_follower(209059, 70, 2515);
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
    let started = crate::spell_cast::now(app.world());
    finish_follower_precombat(&mut app, actor);
    assert!(
        crate::spell_cast::now(app.world()) - started > 3.0,
        "the Fire precast must complete before the measurement starts"
    );
    assert!(
        app.world()
            .get::<server_follower_apl::FollowerApl>(actor)
            .unwrap()
            .in_combat()
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(1459 | 55342 | 11366))),
        "precombat casts must not be counted inside the fight"
    );
    let health = app
        .world()
        .get::<shared::components::Health>(target)
        .unwrap();
    assert!(
        health.current > health.max * 0.99,
        "the fixed-time health timeline must not advance during precombat"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_fire_blast_recharge_refund_preserves_internal_cooldown() {
    let (mut app, actor, target, _) = prepare_follower(209059, 70, 2515);
    assert!(
        app.world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == 342344)
    );
    crate::spell_cast::creature_cast(app.world_mut(), actor, 108853, Some(target), false).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("fire_blast".into(), 108853)].into(),
        ..default()
    };
    let facts = crate::simc_apl_native::NativeFacts::new(
        app.world(),
        actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 120.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    );
    let charges =
        server::simc_apl::Facts::value(&facts, "cooldown.fire_blast.charges_fractional").unwrap();
    assert!(
        charges > 2.01,
        "the actual From the Ashes hit must advance recharge: {charges}"
    );
    let start = crate::spell_cast::history_now(app.world());
    assert!(matches!(
        crate::spell_cast::creature_cast(app.world_mut(), actor, 108853, Some(target), false),
        Err(crate::spell_cast::CastFailure::OnCooldown)
    ));
    while crate::spell_cast::history_now(app.world()) - start < 0.45 {
        app.update();
    }
    assert!(matches!(
        crate::spell_cast::creature_cast(app.world_mut(), actor, 108853, Some(target), false),
        Err(crate::spell_cast::CastFailure::OnCooldown)
    ));
    while crate::spell_cast::history_now(app.world()) - start < 0.55 {
        app.update();
    }
    crate::spell_cast::creature_cast(app.world_mut(), actor, 108853, Some(target), false).unwrap();
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_fireball_master_of_flame_depends_on_combustion() {
    fn fireball_damage(selected: bool, combustion: bool) -> i32 {
        let _seed = crate::random::seed_scope(70448);
        let (mut app, actor, target, _) = prepare_follower(209059, 70, 2515);
        let known: Vec<_> = app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .iter()
            .filter(|id| selected || *id != 384174)
            .collect();
        app.world_mut()
            .entity_mut(actor)
            .insert(crate::class_progression::KnownSpells::fixture(known));
        for _ in 0..5 {
            app.update();
        }
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * 0.5;
        if combustion {
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 190319, Some(actor), 1);
        }
        crate::spell_cast::creature_cast(app.world_mut(), actor, 133, Some(target), false).unwrap();
        for _ in 0..100 {
            app.update();
        }
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(133)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("ordinary Fireball must hit the target")
            .amount
    }
    for (combustion, multiplier) in [(false, 1.15), (true, 1.0)] {
        let base = fireball_damage(false, combustion) as f32;
        let actual = fireball_damage(true, combustion) as f32;
        assert!(
            (actual / base - multiplier).abs() < 0.002,
            "Master of Flame with combustion={combustion}: {base} -> {actual}, expected {multiplier}x"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_combustion_restores_selected_fire_blast_charge() {
    fn charges_after_combustion(selected: bool) -> f64 {
        let _seed = crate::random::seed_scope(70448);
        let (mut app, actor, target, _) = prepare_follower(209059, 70, 2515);
        // Isolate real casts from both the follower brain and its production APL.
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        let known: Vec<_> = app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .iter()
            .filter(|id| selected || *id != 451875)
            .collect();
        app.world_mut()
            .entity_mut(actor)
            .insert(crate::class_progression::KnownSpells::fixture(known));
        for _ in 0..3 {
            crate::spell_cast::creature_cast(app.world_mut(), actor, 108853, Some(target), false)
                .unwrap();
            for _ in 0..11 {
                app.update();
            }
        }
        let bindings = crate::simc_apl_native::Bindings {
            actions: [("fire_blast".into(), 108853)].into(),
            ..default()
        };
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 120.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        let read = |app: &App| {
            let facts = crate::simc_apl_native::NativeFacts::new(
                app.world(),
                actor,
                &bindings,
                encounter,
                1,
            );
            server::simc_apl::Facts::value(&facts, "cooldown.fire_blast.charges").unwrap()
        };
        assert_eq!(read(&app), 0.0, "all three native charges must be spent");
        crate::spell_cast::creature_cast(app.world_mut(), actor, 190319, Some(actor), false)
            .unwrap();
        read(&app)
    }
    assert_eq!(charges_after_combustion(false), 0.0);
    // This level-70 native starter build selects rank two: two charges.
    assert_eq!(charges_after_combustion(true), 2.0);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_fuel_flamestrike_generates_heating_up_on_one_target() {
    let _seed = crate::random::seed_scope(70448);
    let (mut app, actor, target, _) = prepare_follower(209059, 70, 2515);
    assert!(
        app.world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(416094)
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 190319, Some(actor), 1);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 48108, Some(actor), 1);
    server_follower_apl::literal_rotation::execution::execute(
        app.world_mut(),
        actor,
        server_follower_apl::literal_rotation::execution::Execution::Spell(
            1254851,
            target,
            Some(target),
        ),
    )
    .unwrap();
    assert!(!crate::auras::has_aura(app.world(), actor, 48108));
    assert!(
        crate::auras::has_aura(app.world(), actor, 48107),
        "a one-target Flamestrike during Combustion must grant Heating Up from Fuel the Fire"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_dummy_uses_class_apl_to_cast() {
    let (mut app, actor, target, _) = prepare_follower(209059, 70, 2515);
    assert!(
        app.world()
            .get::<crate::followers::AplControlled>(actor)
            .is_some()
    );
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
    for _ in 0..400 {
        app.update();
    }
    assert!(
        app.world()
            .get::<server_follower_apl::FollowerApl>(actor)
            .is_some()
    );
    assert!(
        app.world()
            .get::<server_follower_apl::FollowerAplFailed>(actor)
            .is_none()
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(133 | 11366)))
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB and FOLLOWER_DPS_TANK_TRACE"]
fn follower_class_tank_replays_incoming_after_apl_precombat() {
    let (mut app, actor, target, _) = prepare_follower(209057, 70, 2515);
    let trace = std::env::var_os("FOLLOWER_DPS_TANK_TRACE").expect("tank trace");
    let mut scenario = super::follower_tank_scenario::TankScenario::read(
        std::path::Path::new(&trace),
        app.world_mut(),
        target,
        actor,
        70,
    )
    .unwrap();
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
    finish_follower_precombat(&mut app, actor);
    for frame in 1..=120 {
        scenario.advance(app.world_mut(), frame * 50).unwrap();
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(target.to_bits())
                && event.target == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the class tank must receive the recorded SimC dummy attacks"
    );
    assert!(
        app.world()
            .get::<server_follower_apl::FollowerAplFailed>(actor)
            .is_none()
    );
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

fn sample_actor_auras(
    world: &World,
    actor: Entity,
    samples: &mut std::collections::BTreeMap<u32, u32>,
) {
    let Some(auras) = world.get::<crate::auras::ActiveAuras>(actor) else {
        return;
    };
    let active: std::collections::BTreeSet<u32> = auras.iter().map(|aura| aura.spell_id).collect();
    for spell in active {
        *samples.entry(spell).or_default() += 1;
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_aura_sampling_tracks_actual_active_duration() {
    let (mut app, actor, _, _) = prepare_follower(209059, 70, 2515);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 190319, Some(actor), 1);
    let mut samples = std::collections::BTreeMap::new();
    for _ in 0..20 {
        app.update();
        sample_actor_auras(app.world(), actor, &mut samples);
    }
    assert_eq!(samples.get(&190319), Some(&20));
    for _ in 0..240 {
        app.update();
    }
    sample_actor_auras(app.world(), actor, &mut samples);
    assert_eq!(
        samples.get(&190319),
        Some(&20),
        "expired auras must not gain uptime"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_sentinel_starts_full_and_decay_follows_simc() {
    fn fixture() -> (App, Entity, Entity) {
        let (mut app, actor, target, _) = prepare_follower(209057, 70, 2515);
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::AplControlled>();
        for _ in 0..5 {
            app.update();
        }
        crate::spell_cast::creature_cast(app.world_mut(), actor, 389539, Some(actor), false)
            .unwrap();
        (app, actor, target)
    }
    fn stacks(app: &App, actor: Entity) -> u8 {
        app.world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 389539 && aura.caster == actor)
            .unwrap()
            .stacks
    }
    let (mut app, actor, _) = fixture();
    assert_eq!(stacks(&app, actor), 15);
    for _ in 0..22 {
        app.update();
    }
    assert_eq!(stacks(&app, actor), 14);
    for _ in 0..25 {
        app.update();
    }
    assert_eq!(
        stacks(&app, actor),
        14,
        "SimC decay cannot retrigger while its own cooldown runs"
    );

    let (mut app, actor, target) = fixture();
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::HolyPower)
        .unwrap()
        .current = 3.0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 53600, Some(target), false).unwrap();
    for _ in 0..22 {
        app.update();
    }
    assert_eq!(
        stacks(&app, actor),
        15,
        "paid spender delays the first decay"
    );
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(stacks(&app, actor), 14);

    let (mut app, actor, target) = fixture();
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::HolyPower)
        .unwrap()
        .current = 3.0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 53600, Some(target), false).unwrap();
    app.world_mut()
        .resource_mut::<Time<bevy::time::Virtual>>()
        .pause();
    for _ in 0..50 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<Time<bevy::time::Virtual>>()
        .unpause();
    for _ in 0..22 {
        app.update();
    }
    assert_eq!(
        stacks(&app, actor),
        15,
        "paused real time must not advance the aura decay clock"
    );
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(stacks(&app, actor), 14);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_survives_sentinel_expiry_at_one_health() {
    let (mut app, actor, target, _) = prepare_follower(209057, 70, 2515);
    for _ in 0..5 {
        app.update();
    }
    crate::spell_cast::creature_cast(app.world_mut(), actor, 389539, Some(actor), false).unwrap();
    crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
    app.world_mut()
        .entity_mut(actor)
        .insert(server_follower_apl::FollowerAplFailed);
    for _ in 0..220 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 389539));
    app.world_mut()
        .get_mut::<shared::components::Health>(actor)
        .unwrap()
        .current = 1.0;
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::HolyPower)
        .unwrap()
        .current = 3.0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 53600, Some(target), false).unwrap();
    for _ in 0..30 {
        app.world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap()
            .current = 1.0;
        app.update();
        if !crate::auras::has_aura(app.world(), actor, 389539) {
            break;
        }
    }
    assert!(
        !crate::auras::has_aura(app.world(), actor, 389539),
        "Sentinel must expire"
    );
    assert_eq!(
        app.world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current,
        1.0
    );
    assert!(
        crate::auras::has_aura(app.world(), actor, 132403),
        "expiration must not purge Shield of the Righteous"
    );
}

fn trace_sotr_removal(
    world: &mut World,
    _: Entity,
    aura: &crate::auras::AuraInstance,
    reason: crate::auras::RemoveReason,
) {
    eprintln!(
        "FOLLOWER_SOTR_REMOVED {}",
        json!({"reason": format!("{reason:?}"), "remaining_s": aura.expires_at.map(|expiry| expiry - crate::spell_cast::history_now(world))})
    );
}

fn self_absorb_receipts(world: &World, actor: Entity) -> (BTreeMap<u32, f64>, f64, f64) {
    let Some(capture) = world.get_resource::<server::test_support::healer_intake::Capture>() else {
        return (BTreeMap::new(), 0.0, 0.0);
    };
    let spells = capture
        .consumed_absorbs_by_spell
        .iter()
        .filter_map(|(&(caster, spell), &amount)| (caster == actor).then_some((spell, amount)))
        .collect();
    let usage = capture
        .consumed_absorbs
        .get(&actor)
        .copied()
        .unwrap_or_default();
    (spells, usage.non_unit, usage.unknown_scale)
}

#[test]
fn follower_tank_reports_only_the_actors_consumed_absorbs() {
    let mut app = App::new();
    let actor = app.world_mut().spawn_empty().id();
    let other = app.world_mut().spawn_empty().id();
    app.insert_resource(server::test_support::healer_intake::Capture {
        members: vec![actor],
        active_hit_scale: Some((actor, 1.0)),
        ..Default::default()
    });
    server::test_support::healer_intake::record_absorb(
        app.world_mut(),
        actor,
        actor,
        None,
        1_266_619,
        125.0,
    );
    server::test_support::healer_intake::record_absorb(
        app.world_mut(),
        other,
        actor,
        None,
        17,
        50.0,
    );
    let (spells, non_unit, unknown) = self_absorb_receipts(app.world(), actor);
    assert_eq!(spells, [(1_266_619, 125.0)].into());
    assert_eq!((non_unit, unknown), (0.0, 0.0));
}

/// SimC76f8118 sc_monk.cpp7245–7252 trace: each dodged dummy swing reduces every
/// running Brew by the 268/70 profile's rank-2 Anvil and Stave value, 1s.
#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_brewmaster_avoidance_reduces_running_brews() {
    use server::test_support::healer_intake::{Hit, HitKind};
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let _seed = crate::random::seed_scope(70448);
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 268, 70);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 115203, Some(actor), false).unwrap();
    app.update();
    let remaining = |app: &App| {
        let world = app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(115203)
            .unwrap();
        world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world))
    };
    let swing = |outcome: &str| Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 1_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: outcome.into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    let mut spent = |outcome: &str| {
        let before = remaining(&app);
        crate::combat::replay_healer_hit(app.world_mut(), target, actor, &swing(outcome)).unwrap();
        app.update();
        before - remaining(&app)
    };
    // A landed swing only advances the running Brew by one frame.
    let frame = spent("Hit");
    for outcome in ["Dodge", "Miss"] {
        let reduction = spent(outcome) - frame;
        assert!(
            (999..=1001).contains(&reduction),
            "{outcome} reduced the running Brew by {reduction}ms"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_attacks_with_its_equipped_weapon_type() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    for (spec, ranged) in [(253, true), (73, false)] {
        let _seed = crate::random::seed_scope(70448);
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, spec, 70);
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        for _ in 0..5 {
            app.update();
        }
        app.world_mut().entity_mut(owner).insert((
            crate::networking::CombatTarget(Some(target)),
            crate::combat::AutoAttack::new(target),
        ));
        crate::combat_refs::set_in_combat_with(app.world_mut(), owner, target);
        finish_follower_precombat(&mut app, actor);
        let stats = app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap();
        assert_eq!(
            stats.main_hand.kind == shared::components::WeaponType::Ranged,
            ranged,
            "spec {spec} must use the expected DB2 weapon type"
        );
        app.world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        for _ in 0..400 {
            app.update();
        }
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let damage: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
            })
            .collect();
        let melee = damage
            .iter()
            .filter(|event| event.spell_id.is_none())
            .count();
        if ranged {
            assert_eq!(melee, 0, "ranged follower must not melee");
            assert!(
                damage.iter().any(|event| event.spell_id == Some(75)),
                "ranged follower must use Auto Shot"
            );
        } else {
            assert!(melee > 0, "melee follower must land melee attacks");
        }
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn frost_follower_comet_storm_requires_ray_and_deals_projectile_damage() {
    let _seed = crate::random::seed_scope(1);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 64, 70);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for frame in 0..FIXED_FIGHT_FRAMES {
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current =
            health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        app.update();
    }
    assert!(
        app.world()
            .get::<server_follower_apl::FollowerAplFailed>(actor)
            .is_none()
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let casts = |id| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(id)
            })
            .count()
    };
    let rays = casts(205021);
    let comets = casts(153595);
    assert!(
        rays > 0 && comets > 0 && comets <= rays,
        "each Comet Storm must be earned from Ray of Frost: ray={rays}, comet={comets}"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(153596)
                && event.amount > 0),
        "Comet Storm must deliver its native projectile damage"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn frost_follower_seed_seven_continues_after_reactive_passive_acquisition() {
    for (level, map) in [(70, 2515), (90, 2813)] {
        let _seed = crate::random::seed_scope(7);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, map).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 64, level);
        let owner = app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .owner;
        for _ in 0..5 {
            app.update();
        }
        start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
        for _ in 0..FIXED_FIGHT_FRAMES {
            app.update();
        }
        assert!(
            app.world()
                .get::<server_follower_apl::FollowerAplFailed>(actor)
                .is_none(),
            "Frost level {level} must not fail after tracked aura acquisition"
        );
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "Frost level {level} must damage its target"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn balance_follower_casts_fury_of_elune_and_continues_rotation() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 102, 70);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for frame in 0..FIXED_FIGHT_FRAMES {
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current =
            health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        app.update();
    }
    assert!(
        app.world()
            .get::<server_follower_apl::FollowerAplFailed>(actor)
            .is_none()
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(1233272 | 1233346))),
        "the actual Balance follower must cast its Eclipse action"
    );
    assert!(
        events
            .iter()
            .any(|event| event.target == Some(actor.to_bits())
                && event.kind == CombatLogKind::AuraApplied
                && matches!(event.spell_id, Some(48517 | 48518))),
        "Eclipse must apply a real Solar or Lunar buff"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(202497)
                && event.amount > 0),
        "Shooting Stars must damage an affected target"
    );
    assert!(
        events
            .iter()
            .any(|event| event.target == Some(actor.to_bits())
                && event.kind == CombatLogKind::Energize
                && event.spell_id == Some(202497)
                && event.amount > 0),
        "Shooting Stars must generate Astral Power"
    );
    let fury = events
        .iter()
        .position(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(202770)
        })
        .expect("the actual follower must cast Fury of Elune");
    assert!(
        events[fury + 1..]
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id != Some(202770)),
        "the rotation must continue after Fury of Elune"
    );
    assert!(
        events[fury + 1..]
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(211545)
                && event.amount > 0),
        "the real Fury field must damage its target with native child 211545"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn balance_follower_astral_smolder_deals_periodic_damage() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 102, 90);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for _ in 0..FIXED_FIGHT_FRAMES {
        app.update();
    }
    assert!(
        app.world()
            .get::<server_follower_apl::FollowerAplFailed>(actor)
            .is_none(),
        "the rotation must continue after reactive buffs appear"
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.target == Some(actor.to_bits())
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
                && event.spell_id == Some(450360)),
        "the real follower must proc Touch the Cosmos from generators"
    );
    assert!(
        events
            .iter()
            .any(|event| event.target == Some(actor.to_bits())
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
                && event.spell_id == Some(1263382)),
        "Eclipse must grant Ascendant Stars"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1271222)
                && event.amount > 0),
        "actual Starfire hits must produce selected Star Cascade damage"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(394111)
                && event.amount > 0),
        "Sundered Firmament must damage the actual target"
    );
    assert!(
        events
            .iter()
            .any(|event| event.target == Some(actor.to_bits())
                && event.kind == CombatLogKind::Energize
                && event.spell_id == Some(394108)
                && event.amount > 0),
        "Sundered Firmament must generate Astral Power"
    );
    let lunar_bolts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1263137)
        })
        .collect();
    assert!(
        lunar_bolts.len() >= 3
            && lunar_bolts
                .iter()
                .all(|event| event.crit && event.amount > 0),
        "Lunar Eclipse must launch critical Lunar Bolt damage"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1261573)
                && event.crit
                && event.amount > 0),
        "Solar Eclipse must launch critical Solar Bolt damage"
    );
    let umbral_applications = events
        .iter()
        .filter(|event| {
            event.target == Some(actor.to_bits())
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
                && event.spell_id == Some(393763)
        })
        .count();
    let generators = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(194153 | 190984))
        })
        .count();
    assert!(
        umbral_applications > 0 && umbral_applications < generators,
        "Umbral procs {umbral_applications} must be intermittent across {generators} generators"
    );
    assert!(
        events
            .iter()
            .any(|event| event.target == Some(actor.to_bits())
                && event.kind == CombatLogKind::AuraRemoved
                && event.spell_id == Some(393763)),
        "a generator must consume Umbral Embrace"
    );
    let fires_applications = events
        .iter()
        .filter(|event| {
            event.target == Some(actor.to_bits())
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
                && event.spell_id == Some(1263363)
        })
        .count();
    let eclipse_casts = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(
                    event.spell_id,
                    Some(1233272 | 1233346 | 102560 | 194223 | 390414)
                )
        })
        .count();
    assert!(
        fires_applications > 0 && fires_applications <= eclipse_casts,
        "Ascendant Fires applications {fires_applications} must come from {eclipse_casts} Eclipse entries"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastStart
                && event.spell_id == Some(194153)),
        "Starfire must return to a non-instant cast after Ascendant Fires is consumed"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1263250)
                && event.amount > 0),
        "Eclipse critical hits must produce Astral Smolder periodic damage"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn balance_eclipse_grants_decaying_critical_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 102, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::AplControlled>();
    for _ in 0..5 {
        app.update();
    }
    let starfire = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(194153)
        .unwrap()
        .clone();
    let eclipse = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1233272)
        .unwrap()
        .clone();
    let before = crate::spell_cast::effects::crit_chance(app.world(), actor, target, &starfire);
    crate::spell_cast::execute_cast(app.world_mut(), actor, &eclipse, Some(actor)).unwrap();
    let stacks = |app: &App| {
        app.world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 394050)
            .map_or(0, |aura| aura.stacks)
    };
    assert_eq!(stacks(&app), 10, "Lunar Eclipse grants ten native stacks");
    let initial = crate::spell_cast::effects::crit_chance(app.world(), actor, target, &starfire);
    assert!(
        initial > before + 15.0,
        "Eclipse stacks increase Arcane crit: {before} -> {initial}"
    );
    for _ in 0..22 {
        app.update();
    }
    assert_eq!(
        stacks(&app),
        9,
        "one stack decays at the native one-second period"
    );
    let decayed = crate::spell_cast::effects::crit_chance(app.world(), actor, target, &starfire);
    assert!(
        decayed < initial,
        "critical chance follows stack decay: {initial} -> {decayed}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn balance_ascendant_stars_boosts_three_starsurges() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 102, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::AplControlled>();
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = -100.0;
    let spell = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(78674)
        .unwrap()
        .clone();
    let power = shared::components::PowerType::LunarPower;
    let fire = |app: &mut App| {
        app.world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = -100.0;
        app.world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap()
            .get_mut(power)
            .unwrap()
            .current = 1000.0;
        crate::spell_cast::execute_cast(app.world_mut(), actor, &spell, Some(target)).unwrap();
        for _ in 0..20 {
            app.update();
        }
    };
    fire(&mut app);
    crate::spell_cast::cast_triggered_with_overrides(
        app.world_mut(),
        actor,
        1263382,
        Some(actor),
        1,
        crate::spell_cast::TriggeredSpellOverrides {
            aura_stacks: Some(3),
            ..Default::default()
        },
    );
    let initial = app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1263382)
        .map_or(0, |aura| aura.stacks);
    assert_eq!(initial, 3, "fixture must start with three stacks");
    for remaining in (0..3).rev() {
        fire(&mut app);
        let stacks = app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1263382)
            .map_or(0, |aura| aura.stacks);
        assert_eq!(stacks, remaining, "one stack must be consumed per cast");
    }
    assert!(
        !crate::auras::has_aura(app.world(), actor, 1263382),
        "three Starsurges consume three stacks"
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(78674)
        })
        .map(|event| event.amount)
        .collect();
    assert_eq!(hits.len(), 4, "one damage result per Starsurge: {hits:?}");
    for hit in &hits[1..] {
        assert!(
            *hit > hits[0] * 11 / 10,
            "Ascendant Stars amplifies each spender: {hits:?}"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn balance_touch_cosmos_makes_one_starsurge_free() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 102, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::AplControlled>();
    for _ in 0..5 {
        app.update();
    }
    let power = shared::components::PowerType::LunarPower;
    app.world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(power)
        .unwrap()
        .current = 800.0;
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 450360, Some(actor), 1);
    let starsurge = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(78674)
        .unwrap()
        .clone();
    crate::spell_cast::execute_cast(app.world_mut(), actor, &starsurge, Some(target)).unwrap();
    let after = crate::spell_cast::powers::power_value(app.world(), actor, power)
        .unwrap()
        .0;
    assert_eq!(after, 800.0, "Touch the Cosmos must make Starsurge free");
    assert!(
        !crate::auras::has_aura(app.world(), actor, 450360),
        "the free cast consumes the buff"
    );
    for _ in 0..40 {
        app.update();
    }
    let before = crate::spell_cast::powers::power_value(app.world(), actor, power)
        .unwrap()
        .0;
    crate::spell_cast::execute_cast(app.world_mut(), actor, &starsurge, Some(target)).unwrap();
    let after = crate::spell_cast::powers::power_value(app.world(), actor, power)
        .unwrap()
        .0;
    assert_eq!(
        before - after,
        360.0,
        "the next Starsurge must pay its modified native AP cost"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn balance_umbral_starfire_amplifies_only_during_eclipse() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 102, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::AplControlled>();
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = -100.0;
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 48518, Some(actor), 1);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 194153, Some(target), 1);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 393763, Some(actor), 1);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 194153, Some(target), 1);
    crate::auras::remove_auras(
        app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, 48517 | 48518),
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 194153, Some(target), 1);
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(194153)
        })
        .map(|event| event.amount)
        .collect();
    assert_eq!(hits.len(), 3, "one Starfire hit per cast: {hits:?}");
    assert!(
        hits[1] > hits[0] * 3 / 2,
        "Umbral must amplify Starfire: {hits:?}"
    );
    assert!(
        hits[2] < hits[1],
        "the same buff must not amplify outside Eclipse: {hits:?}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn fury_of_elune_pulses_the_field_and_stops_after_expiry() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 102, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::AplControlled>();
    let second = spawn_level_dummy(&mut app, actor, 70, 1).unwrap();
    let position = *app
        .world()
        .get::<shared::components::Position>(target)
        .unwrap();
    app.world_mut()
        .entity_mut(second)
        .insert(shared::components::Position {
            x: position.x + 1.0,
            ..position
        });
    crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
    crate::combat_refs::set_in_combat_with(app.world_mut(), actor, second);
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(app.world_mut(), actor, 202770, Some(target), false).unwrap();
    let hits = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(211545)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    for _ in 0..4 {
        app.update();
    }
    assert!(
        hits(&app).is_empty(),
        "no immediate pulse before the hasted 500ms period"
    );
    for _ in 0..36 {
        app.update();
    }
    let first = hits(&app);
    assert!(
        first
            .iter()
            .any(|event| event.target == Some(target.to_bits()))
    );
    assert!(
        first
            .iter()
            .any(|event| event.target == Some(second.to_bits()))
    );
    let primary = first
        .iter()
        .find(|event| event.target == Some(target.to_bits()) && !event.crit)
        .unwrap();
    let secondary = first
        .iter()
        .find(|event| event.target == Some(second.to_bits()) && !event.crit)
        .unwrap();
    let hit_targets: std::collections::HashSet<_> =
        first.iter().filter_map(|event| event.target).collect();
    let reduction = (1.0 / hit_targets.len() as f32).sqrt();
    assert!(
        (secondary.amount as f32 / primary.amount as f32 - reduction).abs() < 0.03,
        "secondary hits receive the source one-target soft cap: primary={primary:?}, secondary={secondary:?}, targets={hit_targets:?}"
    );
    app.world_mut()
        .entity_mut(second)
        .insert(shared::components::Position {
            x: position.x + 100.0,
            ..position
        });
    let before = first
        .iter()
        .filter(|event| event.target == Some(second.to_bits()))
        .count();
    for _ in 0..150 {
        app.update();
    }
    let after = hits(&app);
    assert_eq!(
        after
            .iter()
            .filter(|event| event.target == Some(second.to_bits()))
            .count(),
        before,
        "units outside the fixed field take no further pulses"
    );
    let total = after.len();
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        hits(&app).len(),
        total,
        "the field stops pulsing after its native duration"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn fixed_fight_seed_replays_identical_events_and_outcome() {
    fn replay() -> (serde_json::Value, Vec<serde_json::Value>) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "dungeon_simulator::dps_parity::follower_dps_diagnostic::follower_dps_production_dummy",
                "--exact", "--ignored", "--nocapture", "--test-threads=1",
            ])
            .env("FOLLOWER_DPS_SPEC", "62")
            .env("FOLLOWER_DPS_LEVEL", "90")
            .env("FOLLOWER_DPS_MAP", "2813")
            .env("FOLLOWER_DPS_SEED", "8")
            .env("FOLLOWER_TRACE_EVENTS", "1")
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let report: serde_json::Value = stdout
            .lines()
            .find_map(|line| {
                line.split_once("FOLLOWER_DPS ")
                    .map(|(_, value)| serde_json::from_str(value).unwrap())
            })
            .expect("DPS report");
        let stderr = String::from_utf8(output.stderr).unwrap();
        let events: Vec<serde_json::Value> = stderr
            .lines()
            .filter_map(|line| line.strip_prefix("FOLLOWER_REPLAY_EVENT "))
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(!events.is_empty(), "fight emitted no events");
        (report, events)
    }
    let first = replay();
    let second = replay();
    for (index, (left, right)) in first.1.iter().zip(&second.1).enumerate() {
        assert_eq!(left, right, "combat event {index} diverged");
    }
    assert_eq!(
        first.1.len(),
        second.1.len(),
        "combat event counts diverged"
    );
    assert_eq!(first.0, second.0, "fight outcomes diverged");
}

#[test]
#[ignore = "production follower single-target throughput diagnostic"]
fn follower_dps_production_dummy() {
    let entry: Option<u32> = std::env::var("FOLLOWER_DPS_ENTRY")
        .ok()
        .map(|entry| entry.parse().expect("numeric entry"));
    let level: u8 = std::env::var("FOLLOWER_DPS_LEVEL")
        .unwrap()
        .parse()
        .unwrap();
    let map: u32 = std::env::var("FOLLOWER_DPS_MAP").unwrap().parse().unwrap();
    let seed: u64 = std::env::var("FOLLOWER_DPS_SEED").unwrap().parse().unwrap();
    let _seed = crate::random::seed_scope(seed);
    let (mut app, actor, target, profile) = if let Ok(spec) = std::env::var("FOLLOWER_DPS_SPEC") {
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, map).unwrap();
        prepare_class_follower(app, copy, spec.parse().expect("numeric spec"), level)
    } else {
        prepare_follower(
            entry.expect("FOLLOWER_DPS_ENTRY or FOLLOWER_DPS_SPEC"),
            level,
            map,
        )
    };
    if std::env::var_os("FOLLOWER_AUDIT_DEATH").is_some() {
        app.init_resource::<crate::auras::DeathAuraAudit>();
    }
    if std::env::var_os("FOLLOWER_TRACE_ACTIONS").is_some() {
        let mut scripts = app
            .world_mut()
            .resource_mut::<crate::auras::scripts::AuraScripts>();
        let script = scripts.0.entry(132403).or_default();
        assert!(script.on_remove.is_none());
        script.on_remove = Some(trace_sotr_removal);
    }
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
    start_fixed_time_fight(
        &mut app,
        owner,
        actor,
        target,
        f64::from(FIXED_FIGHT_FRAMES) / 20.0,
    );
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
    let trace_actions = std::env::var_os("FOLLOWER_TRACE_ACTIONS").is_some();
    let mut stat_sums = [0.0f64; 6];
    let mut aura_frames = std::collections::BTreeMap::new();
    let mut target_aura_frames = std::collections::BTreeMap::new();
    let mut mark_trace = Vec::new();
    let mut previous_marks = (false, false);
    for frame in 0..FIXED_FIGHT_FRAMES {
        if let Some(scenario) = &mut tank_scenario {
            scenario
                .advance(app.world_mut(), u64::from(frame + 1) * 50)
                .expect("replay SimC tank pressure");
        }
        // SimC fixed-time Patchwerk uses a 100% to 0% target-health timeline.
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current =
            health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        let before_power = crate::spell_cast::powers::power_value(
            app.world(),
            actor,
            shared::components::PowerType::HolyPower,
        );
        app.update();
        super::super::healer_simc_replay::report_death_audit(&mut app, &[actor, owner]);
        let world = app.world();
        sample_actor_auras(world, actor, &mut aura_frames);
        if let Some(auras) = world.get::<crate::auras::ActiveAuras>(target) {
            for aura in auras.iter().filter(|aura| aura.caster == actor) {
                *target_aura_frames.entry(aura.spell_id).or_insert(0u32) += 1;
            }
        }
        if trace_actions {
            let marks = (
                crate::auras::has_aura(world, target, 1253601),
                crate::auras::has_aura(world, target, 1301098),
            );
            if marks != previous_marks {
                mark_trace.push(((frame + 1) as f64 / 20.0, marks));
                previous_marks = marks;
            }
        }
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
        let gcd = world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .is_some_and(|history| history.has_active_gcd(now));
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
        if std::env::var_os("FOLLOWER_TRACE_EVENTS").is_some() {
            for event in &events[seen_events..] {
                eprintln!(
                    "FOLLOWER_REPLAY_EVENT {}",
                    json!({"tick": frame + 1, "event": event})
                );
            }
        }
        if trace_actions {
            let after_power = crate::spell_cast::powers::power_value(
                app.world(),
                actor,
                shared::components::PowerType::HolyPower,
            );
            for event in &events[seen_events..] {
                if event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
                {
                    let sotr = app
                        .world()
                        .get::<crate::auras::ActiveAuras>(actor)
                        .and_then(|auras| {
                            auras
                                .iter()
                                .find(|aura| aura.spell_id == 132403 && aura.caster == actor)
                        });
                    cast_trace.push(
                        json!({"at_s": (frame + 1) as f64 / 20.0, "spell": event.spell_id,
                        "power_before": before_power, "power_after": after_power,
                        "sotr_remaining_s": sotr.and_then(|aura| aura.expires_at).map(|expiry| expiry - crate::spell_cast::history_now(app.world())),
                        "sotr_duration_ms": sotr.map(|aura| aura.duration_ms),
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
    let mut energize_details = BTreeMap::<String, [i64; 3]>::new();
    let expected_avoidance = std::env::var_os("FOLLOWER_DPS_EXPECTED_AVOIDANCE").is_some();
    let incoming: Vec<_> = events
        .iter()
        .filter(|event| {
            event.target == Some(actor.to_bits())
                && (event.kind == CombatLogKind::Damage
                    || (expected_avoidance
                        && event.source == Some(target.to_bits())
                        && event.spell_id.is_none()))
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
            let detail = energize_details
                .entry(format!("{id}:{}", event.amount))
                .or_default();
            detail[0] += 1;
            detail[1] += i64::from(event.amount);
            detail[2] += i64::from(event.overflow);
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
            .get::<server_follower_apl::FollowerAplFailed>(actor)
            .is_none(),
        "follower APL failed during the dummy measurement"
    );
    assert!(
        app.world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current
            > 0.0,
        "follower died before completing the dummy measurement"
    );
    if let Some(capture) = app
        .world()
        .get_resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
    {
        println!("FOLLOWER_FACTORS {}", capture.export());
    }
    let (self_absorbs, absorb_non_unit, absorb_unknown_scale) =
        self_absorb_receipts(app.world(), actor);
    let target_uptime: std::collections::BTreeMap<_, _> = target_aura_frames
        .into_iter()
        .map(|(spell, frames)| {
            (
                spell,
                f64::from(frames) * 100.0 / f64::from(FIXED_FIGHT_FRAMES),
            )
        })
        .collect();
    println!("FOLLOWER_TARGET_AURAS {}", serde_json::json!(target_uptime));
    if trace_actions {
        println!("FOLLOWER_MARK_TRACE {}", serde_json::json!(mark_trace));
    }
    let mut receipt = json!({"entry":entry,"spec":profile.spec_id,"level":level,"map":map,"difficulty":205,"seed":seed,"seconds":120,"sheet":sheet,
        "self_absorbs_by_spell":self_absorbs,"self_absorb_non_unit":absorb_non_unit,"self_absorb_unknown_scale":absorb_unknown_scale,
        "items":items,"modifiers":modifiers,"talents":talents,"known_spells":known_spells,"target_armor":armor,"checkpoints":checkpoints,"incoming":incoming,"actor_id":actor.to_bits(),"target_id":target.to_bits(),"damage":damage,"hits":hits,"hit_breakdown":hit_breakdown,
        "mean_stats":stat_sums.map(|sum| sum / f64::from(FIXED_FIGHT_FRAMES)),"sources":damage_sources,"casts":casts,"cast_trace":cast_trace,"aura_applications":aura_applications,"aura_uptime_pct":aura_frames.into_iter().map(|(spell, frames)| (spell, f64::from(frames) * 100.0 / f64::from(FIXED_FIGHT_FRAMES))).collect::<std::collections::BTreeMap<_, _>>(),"energize":energize,
        "flame_shock_s":flame_shock_frames as f64 / 20.0,"lava_surge_s":lava_surge_frames as f64 / 20.0,
        "storm_elemental_s":storm_frames as f64 / 20.0,"lava_casts_started_during_storm":lava_during_storm,
        "bolt_casts_started_during_storm":bolt_during_storm,"lava_successes_during_stable_storm":lava_during_stable_storm,"casting_s":casting as f64 / 20.0,
        "gcd_s":gcd_total as f64 / 20.0,"gcd_only_s":gcd_only as f64 / 20.0,"ready_s":ready as f64 / 20.0});
    receipt["energize_details"] = json!(energize_details);
    println!("FOLLOWER_DPS {receipt}");
}
