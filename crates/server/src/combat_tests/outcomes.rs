//! Swing outcomes reach clients as their `CombatEvent` type
//! (`SMSG_ATTACKER_STATE_UPDATE` `HitInfo`), and creatures roll the hit table.

use super::*;
use crate::item_stats::WeaponDamage;
use crate::melee::PlayerDefense;
use shared::components::WeaponType;

const BLADE: WeaponDamage = WeaponDamage {
    min: 10.0,
    max: 10.0,
    speed: 2.0,
    kind: WeaponType::OneHand,
};

/// `swings` swings of `attacker`'s auto-attack at the target, each through
/// `auto_attack_system`; the event types logged, in order.
fn swing_events(app: &mut App, attacker: Entity, swings: usize) -> Vec<CombatEventType> {
    for _ in 0..swings {
        app.world_mut()
            .get_mut::<AutoAttack>(attacker)
            .unwrap()
            .timer = 0.0;
        app.world_mut()
            .run_system_once(auto_attack_system)
            .expect("auto-attack system should run");
        app.update();
    }
    app.world()
        .resource::<CombatLog>()
        .range(0.0, f64::MAX)
        .iter()
        .map(|entry| entry.event.event_type.clone())
        .collect()
}

fn player_target(app: &mut App, defense: PlayerDefense) -> Entity {
    app.world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            // These defense/CC controls face the attacker at the origin.
            shared::components::Rotation {
                x: 0.0,
                y: std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            Health {
                current: 1.0e6,
                max: 1.0e6,
            },
            Player {
                name: "Victim".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            CharacterStats {
                defense,
                ..default()
            },
        ))
        .id()
}

/// A player's crit reaches clients as `CriticalHit`, not `MeleeDamage`: with a 100%
/// crit chance every swing that is not missed (5%) crits.
#[test]
fn a_players_crit_is_sent_as_a_critical_hit() {
    let mut app = setup_replication_app();
    let victim = player_target(&mut app, PlayerDefense::default());
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), victim);
    app.world_mut().entity_mut(attacker).insert(CharacterStats {
        crit_pct: 100.0,
        main_hand: BLADE,
        ..default()
    });
    let events = swing_events(&mut app, attacker, 20);
    assert_eq!(events.len(), 20);
    assert!(
        events
            .iter()
            .all(|kind| matches!(kind, CombatEventType::CriticalHit | CombatEventType::Miss)),
        "{events:?}"
    );
    assert!(events.contains(&CombatEventType::CriticalHit));
}

/// A creature swinging at a player rolls the player's avoidance
/// (Unit::RollMeleeOutcomeAgainst for every unit): against a 100% block chance each
/// swing not missed is a `Block`, and against 100% parry a `Parry`.
#[test]
fn a_creature_swing_meets_the_players_block_and_parry() {
    for (defense, expected) in [
        (
            PlayerDefense {
                block: 100.0,
                ..PlayerDefense::default()
            },
            CombatEventType::Block,
        ),
        (
            PlayerDefense {
                parry: 100.0,
                ..PlayerDefense::default()
            },
            CombatEventType::Parry,
        ),
    ] {
        let mut app = setup_replication_app();
        let victim = player_target(&mut app, defense);
        let creature = app
            .world_mut()
            .spawn((
                pos(0.0, 0.0, 0.0),
                AutoAttack::new(victim),
                crate::maps::InMap::continent(0),
                Npc {
                    template_id: 1,
                    name: "Fixture wolf".into(),
                },
            ))
            .id();
        let events = swing_events(&mut app, creature, 20);
        assert_eq!(events.len(), 20);
        assert!(
            events
                .iter()
                .all(|kind| *kind == expected || *kind == CombatEventType::Miss),
            "{events:?}"
        );
        assert!(events.contains(&expected), "{events:?}");
    }
}

/// A casting player avoids nothing (Unit.cpp:2408-2413): the creature's swings land.
#[test]
fn a_casting_player_cannot_parry() {
    let mut app = setup_replication_app();
    let victim = player_target(
        &mut app,
        PlayerDefense {
            parry: 100.0,
            ..PlayerDefense::default()
        },
    );
    app.world_mut()
        .entity_mut(victim)
        .insert(shared::casting::CastState::normal(133, 0, 60.0, true));
    let creature = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            AutoAttack::new(victim),
            crate::maps::InMap::continent(0),
            Npc {
                template_id: 1,
                name: "Fixture wolf".into(),
            },
        ))
        .id();
    let events = swing_events(&mut app, creature, 20);
    assert!(!events.contains(&CombatEventType::Parry), "{events:?}");
    assert!(
        events.iter().any(|kind| matches!(
            kind,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        )),
        "{events:?}"
    );
}

/// A level 10 player's swing at a level 18 creature glances whenever it is not avoided
/// (miss 5%, dodge 15%, parry 18%, then glancing 10% + 10% per level = 90%,
/// Unit.cpp:2440-2450), and its combat log line says so
/// (`HITINFO_GLANCING`, CLEU SWING_DAMAGE `glancing`).
#[test]
fn a_glancing_swing_is_flagged_in_the_combat_log() {
    let Some(mut h) = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1) else {
        return;
    };
    let npc = h.npc;
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(shared::components::UnitLevel(18));
    h.attack(npc);
    h.advance(800);
    let swings: Vec<bool> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage && event.spell_id.is_none()
        })
        .map(|event| event.glancing)
        .collect();
    assert!(!swings.is_empty(), "{:?}", h.inbox.log);
    assert!(swings.iter().all(|&glancing| glancing), "{swings:?}");
}

/// Unit::RollMeleeOutcomeAgainst (Unit.cpp:2422-2424) crits a player who is not standing
/// unless the swing misses, and the swing's `DealDamage` (Unit.cpp:1185-1186) stands
/// the player up: force landed swings so each re-seated player must stand.
#[test]
fn a_creature_swing_crits_a_sitting_player_and_stands_it_up() {
    use shared::components::{PlayerStandState, StandState};
    let mut app = setup_replication_app();
    let victim = player_target(&mut app, PlayerDefense::default());
    let creature = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            AutoAttack::new(victim),
            crate::maps::InMap::continent(0),
            Npc {
                template_id: 1,
                name: "Fixture wolf".into(),
            },
        ))
        .id();
    app.world_mut()
        .entity_mut(creature)
        .insert(crate::auras::AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..Default::default()
        });
    for seated in [StandState::Sit, StandState::Sleep, StandState::Kneel]
        .into_iter()
        .cycle()
        .take(20)
    {
        app.world_mut()
            .entity_mut(victim)
            .insert(PlayerStandState(seated));
        swing_events(&mut app, creature, 1);
        assert_eq!(
            app.world().get::<PlayerStandState>(victim),
            Some(&PlayerStandState(StandState::Stand))
        );
    }
    let events = swing_events(&mut app, creature, 0);
    assert_eq!(events.len(), 20);
    assert!(
        events
            .iter()
            .all(|kind| matches!(kind, CombatEventType::CriticalHit)),
        "{events:?}"
    );
}
