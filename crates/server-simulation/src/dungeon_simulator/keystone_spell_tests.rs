//! A Keystone creature's spells take its map/season ExpectedStat tuning in the
//! headless runtime, which installs server startup's own definitions.
use shared::components::{Npc, Position};

const EARTHWHISPER_TENDER: u32 = 241814;
const EARTH_BOLT: u32 = 1241214;

#[test]
fn class_followers_are_knocked_back_by_fel_nova() {
    let _seed = crate::random::seed_scope(1);
    let mut app = super::runtime::build_app();
    let root = super::graph::extraction::asset_root();
    app.insert_resource(server::test_support::LineOfSight::load(&root.join("los")).unwrap());
    let copy = super::runtime::create_copy(&mut app);
    let actors = super::runtime::spawn_population(&mut app, copy);
    let caster = *actors.values().next().expect("hostile creature spawn");
    let position = *app.world().get::<Position>(caster).unwrap();
    let party = crate::followers::simulator::spawn_keystone_party(
        app.world_mut(),
        copy,
        position,
        90,
        [581, 264, 71, 577, 251],
        2,
    )
    .unwrap();
    crate::spell_cast::creature_cast(app.world_mut(), caster, 1223906, Some(caster), true).unwrap();
    for &unit in &party.members {
        assert!(
            app.world()
                .get::<shared::components::MovementControl>(unit)
                .is_some_and(|control| control.controlled),
            "follower {unit:?} must enter knockback flight"
        );
    }
}

#[test]
fn den_keystone_earth_bolt_uses_seasonal_creature_spell_damage() {
    let _seed = crate::random::seed_scope(1);
    let mut app = super::runtime::build_app();
    let copy = super::runtime::create_map_copy(&mut app, 2825, 23).unwrap();
    let actors = super::runtime::spawn_population(&mut app, copy);
    let party = crate::followers::simulator::spawn_keystone_party(
        app.world_mut(),
        copy,
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        90,
        [66, 105, 262, 63, 253],
        2,
    )
    .unwrap();
    let session = server::test_support::keystone::Session::prepare(
        app.world_mut(),
        &crate::test_data::world_db_path(),
        party.owner,
        2,
        1792454400,
    )
    .unwrap();
    let font = session.font_position(app.world()).unwrap();
    app.world_mut().entity_mut(party.owner).insert(font);
    session.activate(app.world_mut()).unwrap();
    let tender = *actors
        .values()
        .find(|&&actor| {
            app.world()
                .get::<Npc>(actor)
                .is_some_and(|npc| npc.template_id == EARTHWHISPER_TENDER)
        })
        .expect("Den spawns Earthwhisper Tender");
    // 12.1.5 ExpectedStatMod 280 and 448 CreatureSpellDamageMod of Den map
    // tuning 1279 in season 120.
    let multiplier = app
        .world()
        .get::<crate::level_scaling::ScaledStats>(tender)
        .unwrap()
        .0
        .keystone_spell_multiplier(false);
    assert!(
        (multiplier - 1.939_299_9 * 1.621_000_1).abs() < 1e-4,
        "{multiplier}"
    );
    // The Restoration Druid survives a hit and has no magic damage reduction
    // beyond Versatility; 20 yards keeps it out of the Tender's melee.
    let druid = *party
        .members
        .iter()
        .find(|&&unit| {
            app.world()
                .get::<shared::components::ActiveSpec>(unit)
                .is_some_and(|spec| spec.0 == 105)
        })
        .unwrap();
    let at = *app.world().get::<Position>(tender).unwrap();
    app.world_mut().entity_mut(druid).insert(Position {
        x: at.x + 20.0,
        ..at
    });
    let cursor = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(app.world_mut(), tender, EARTH_BOLT, Some(druid), true)
        .unwrap();
    let source = tender.to_bits();
    let target = druid.to_bits();
    let mut hit = None;
    for _ in 0..200 {
        app.update();
        hit = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..]
            .iter()
            .find(|event| {
                event.kind == shared::protocol::CombatLogKind::Damage
                    && event.spell_id == Some(EARTH_BOLT)
                    && event.source == Some(source)
                    && event.target == Some(target)
            })
            .cloned();
        if hit.is_some() {
            break;
        }
    }
    let hit = hit.unwrap_or_else(|| {
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..];
        let related: Vec<_> = events
            .iter()
            .filter(|event| event.source == Some(source) || event.target == Some(target))
            .take(40)
            .collect();
        panic!(
            "Earth Bolt must damage its target: tender at {:?} health {:?}, target at {:?}; events {related:?}",
            app.world().get::<Position>(tender),
            app.world().get::<shared::components::Health>(tender),
            app.world().get::<Position>(druid),
        )
    });
    // BasePoints 12% of ExpectedStat level-90 CreatureSpellDamage 308506.656 × the
    // seasonal mods, at +2 (key damage 1.0); Variance 0.05. WCL BDjZ2QF6pNr8xV4W/1
    // and Den +2 reports record a median unmitigated 116,400.
    let base = (0.12f32 * 308_506.66 * multiplier).round();
    let total = (hit.amount + hit.absorbed + hit.resisted + hit.blocked) as f32;
    // A spell critical strike doubles the hit (Unit::SpellCriticalDamageBonus).
    let dealt = if hit.crit { total / 2.0 } else { total };
    assert!(
        dealt <= base * 1.05 + 1.0 && dealt >= base * 0.85,
        "Earth Bolt dealt {dealt} against a {base} base: {hit:?}"
    );
    println!("EARTH_BOLT base={base} dealt={dealt} event={hit:?}");
}
