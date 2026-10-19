//! TC166/214–215: the source guard acts before real health/death commits.
use super::herald_call::{damage_to_percent, descent_emotes, first_west, reach_first_west};
use super::herald_damage::assert_source_step;
use super::herald_intro::{actors_in_copy, kill_actor, spawn_source_copy};
use super::herald_intro2::{prepared_ramparts, received_casts};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;

const DESCEND: Position = Position {
    x: -1429.1720,
    y: 84.34760,
    z: -1749.1290,
};
const CENTER: Position = Position {
    x: -1410.5204,
    y: 89.05998,
    z: -1740.5852,
};

fn observe_first_west(difficulty: u32) -> (Harness, InMap, Entity, Entity) {
    let (mut h, map, nazan, vazruden) = first_west(difficulty);
    h.app.init_resource::<ObservedCombatEvents>();
    // Type41/TextRange0 has a25yd audience; no NPC position/ownership changes.
    let origin = *h.app.world().get::<Position>(nazan).unwrap();
    let listener = h.player();
    h.app.world_mut().entity_mut(listener).insert(Position {
        x: origin.x + 3.0,
        ..origin
    });
    (h, map, nazan, vazruden)
}

fn reduce_to_one(h: &mut Harness, actor: Entity) {
    let loss = h.health(actor).current - 1.0;
    // Self has exactly the native health scale. This is real nonlethal damage,
    // never a health write or the lethal interceptor itself.
    crate::combat::deal_direct_damage(h.app.world_mut(), actor, actor, loss);
    assert_eq!(h.health(actor).current, 1.0);
}

fn assert_alive_at_one(h: &mut Harness, actor: Entity, map: InMap) {
    assert_eq!(
        h.health(actor).current,
        1.0,
        "source lethal clamp must leave exactly1HP"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(actor)
            .is_none()
    );
    assert!(
        !h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(
                |event| event.target == Some(actor.to_bits()) && event.kind == CombatLogKind::Death
            ),
        "no actual server death event, including ownerless self-source"
    );
    for (_, _, inbox) in &h.players {
        assert!(
            !inbox
                .log
                .iter()
                .any(|event| event.target == Some(actor.to_bits())
                    && event.kind == CombatLogKind::Death),
            "no death publication after intercepted damage"
        );
        assert!(inbox.ends.iter().all(|end| !end.success));
    }
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
}

fn deal_lethal(h: &mut Harness, actor: Entity, attacker: Entity, spell: bool) {
    let amount = h.health(actor).max * 2_000.0;
    if spell {
        // Actual native Fireball effect enters the spell health-commit boundary.
        h.hit(actor, amount);
    } else {
        crate::combat::deal_direct_damage(h.app.world_mut(), attacker, actor, amount);
    }
}

fn assert_airborne_commits(spell: bool) {
    for difficulty in [1, 2] {
        let (mut h, map, nazan, _) = observe_first_west(difficulty);
        let player = h.player();
        deal_lethal(&mut h, nazan, player, spell);
        assert_alive_at_one(&mut h, nazan, map);
        deal_lethal(&mut h, nazan, nazan, false);
        assert_alive_at_one(&mut h, nazan, map);
        if spell {
            deal_lethal(&mut h, nazan, player, true);
        }
        h.advance(0.1);
        assert_alive_at_one(&mut h, nazan, map);
        assert_eq!(
            descent_emotes(&h, nazan),
            1,
            "lethal own damage still starts descent once"
        );
        assert_ne!(h.app.world().get::<Position>(nazan), Some(&DESCEND));
    }
}

#[test]
#[ignore = "RED: Herald source min1 guard missing before direct/self commit"]
fn encounter_coverage_herald_min1_intro_direct_self_nonlethal_and_copy_isolation() {
    for difficulty in [1, 2] {
        let (mut h, map, herald, sentries) = prepared_ramparts(difficulty);
        h.app.init_resource::<ObservedCombatEvents>();
        let other_map = spawn_source_copy(&mut h, 8001, difficulty);
        let other = actors_in_copy(&mut h, 17307, other_map)[0];
        let other_health = h.health(other);
        damage_to_percent(&mut h, herald, 80.0);
        for sentry in sentries {
            kill_actor(&mut h, sentry);
        }
        h.advance(0.05); // Actual Sentry deaths start INTRO1 and stop its patrol.
        let before = *h.app.world().get::<Position>(herald).unwrap();
        let player = h.player();
        deal_lethal(&mut h, herald, player, false);
        assert_alive_at_one(&mut h, herald, map);
        deal_lethal(&mut h, herald, herald, false);
        h.advance(0.05);
        assert_alive_at_one(&mut h, herald, map);
        assert_source_step(&h, herald, before, CENTER);
        assert_eq!(
            descent_emotes(&h, herald),
            0,
            "intro guard must suppress own30 descent"
        );
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Npc>(herald)
                .unwrap()
                .template_id,
            17307
        );
        assert_eq!(h.health(other).current, other_health.current);
        let sentry = actors_in_copy(&mut h, 17517, other_map)[0];
        // An unrelated actor must not acquire this script's protection.
        let amount = h.health(sentry).max * 2_000.0;
        crate::combat::deal_direct_damage(h.app.world_mut(), sentry, sentry, amount);
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(sentry)
                .is_some()
        );
        assert_alive_at_one(&mut h, herald, map);
    }
}

#[test]
#[ignore = "RED: Nazan direct health commit still kills"]
fn encounter_coverage_herald_min1_airborne_direct_and_repeated_self() {
    assert_airborne_commits(false);
}

#[test]
#[ignore = "RED: Nazan spell health commit still kills"]
fn encounter_coverage_herald_min1_airborne_spell_and_repeated_self() {
    assert_airborne_commits(true);
}

fn assert_real_periodic_commits(self_source: bool) {
    for difficulty in [1, 2] {
        let (mut h, map, nazan, _) = observe_first_west(difficulty);
        // Let an actual ongoing Fireball finish; never cancel it to admit the probe.
        for _ in 0..65 {
            if h.app
                .world()
                .get::<shared::casting::CastState>(nazan)
                .is_none()
            {
                break;
            }
            h.advance(0.05);
        }
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(nazan)
                .is_none()
        );
        let caster = if self_source { nazan } else { h.player() };
        let spell_id = if self_source { 28721 } else { 17692 };
        let origin = *h.app.world().get::<Position>(nazan).unwrap();
        if !self_source {
            h.app.world_mut().entity_mut(caster).insert(Position {
                x: origin.x + 3.0,
                ..origin
            });
        }
        let spell = h
            .app
            .world()
            .resource::<SpellStore>()
            .get(spell_id)
            .unwrap();
        // Native Marduk's Curse17692 is magic (not a forced-hit melee probe).
        if !self_source {
            assert_eq!(spell.categories.as_ref().unwrap().defense_type, 1);
        }
        let effect = &spell.effects[0];
        assert_eq!(
            (effect.effect, effect.aura, effect.aura_period_ms),
            (6, 3, 1_000)
        );
        assert!(effect.base_points > 1.0);
        assert!(
            spell.power_costs.is_empty(),
            "actual native probe has no power cost"
        );
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, spell_id, Some(nazan), false)
            .expect("native ordinary periodic probe admission, no trigger/override");
        assert!(h.has_aura(nazan, spell_id));
        reduce_to_one(&mut h, nazan);
        let speed = h
            .app
            .world()
            .get::<crate::creature_ai::SpawnPoint>(nazan)
            .unwrap()
            .speed_run
            * 7.0;
        let travel_time = Vec3::new(
            origin.x - DESCEND.x,
            origin.y - DESCEND.y,
            origin.z - DESCEND.z,
        )
        .length()
            / speed;
        assert!(
            travel_time > 2.05,
            "two native1s ticks must precede point4 arrival: time={travel_time}, speed={speed}, origin={origin:?}"
        );
        for expected_ticks in 1..=2 {
            h.advance(1.0);
            assert_alive_at_one(&mut h, nazan, map);
            let ticks = h
                .app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(caster.to_bits())
                        && event.target == Some(nazan.to_bits())
                        && event.spell_id == Some(spell_id)
                        && event.periodic
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .count();
            assert_eq!(
                ticks, expected_ticks,
                "real authored periodic ticks, not direct flagged damage"
            );
            assert_ne!(h.app.world().get::<Position>(nazan), Some(&DESCEND));
        }
        assert_eq!(received_casts(&h, caster, spell_id), 1);
        assert_eq!(descent_emotes(&h, nazan), 1);
    }
}

#[test]
#[ignore = "RED: periodic damage must use the same precommit guard"]
fn encounter_coverage_herald_min1_real_periodic_external() {
    assert_real_periodic_commits(false);
}

#[test]
#[ignore = "RED: source guard includes self-cast periodic loss"]
fn encounter_coverage_herald_min1_real_periodic_self() {
    assert_real_periodic_commits(true);
}

fn native_avoidance_upper(h: &Harness, player: Entity, target: Entity) -> f64 {
    use crate::melee::{Swinger, Victim, swing_chances};
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(player)
        .unwrap();
    assert!(stats.off_hand.is_none());
    let player_scale = crate::level_scaling::UnitScaling::of(h.app.world(), player);
    let target_scale = crate::level_scaling::UnitScaling::of(h.app.world(), target);
    let difference = i32::from(target_scale.level_against(&player_scale))
        - i32::from(player_scale.level_against(&target_scale));
    let chances = swing_chances(
        Swinger::Player {
            crit_pct: stats.crit_pct,
            dual_wield: false,
        },
        Victim::Creature {
            level_difference: difference,
        },
        true,
    );
    assert_eq!(
        (chances.miss, chances.dodge, chances.parry),
        (5.0, 3.0, 6.0)
    );
    let auras = h.app.world().get::<crate::auras::ActiveAuras>(target);
    for aura in [47, 49, 184] {
        assert_eq!(auras.map_or(0.0, |a| a.modifier(aura)), 0.0);
    }
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(player)
            .map_or(0.0, |m| m.melee_hit_chance_pct),
        0.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(player)
            .map_or(0.0, |a| a.modifier(251)),
        0.0
    );
    let (minimum, _) = crate::melee::weapon_damage_range(
        &stats.main_hand,
        stats.attack_power,
        stats.versatility_pct,
        crate::melee::Hand::Main,
    );
    // Even worst source armor85% and block30% leave a positive native hit.
    assert!((minimum.trunc() * 0.15 * 0.7).trunc() >= 1.0);
    // Front/avoidance-enabled is conservative: direction, casting and native
    // NO_PARRY masks may only reduce these native5+3+6 chances. No RNG override.
    f64::from(chances.miss + chances.dodge + chances.parry) / 100.0
}

#[test]
#[ignore = "RED: real queued native melee commits still kill Nazan"]
fn encounter_coverage_herald_min1_native_melee_copies_before_point4_arrival() {
    for difficulty in [1, 2] {
        let (mut h, map, nazan, _) = observe_first_west(difficulty);
        let player = h.player();
        let avoid = native_avoidance_upper(&h, player, nazan);
        let copies = (1..=16).find(|count| avoid.powi(*count) < 0.001).unwrap();
        // Native miss5%, dodge3%, parry6% => P(no landed main-hand hit) <=
        // 0.14^4 =0.00038416 <1/1000, with one independent ordinary copy each.
        assert_eq!(copies, 4);
        let mut pairs = vec![(map, nazan, player)];
        for index in 1..copies {
            let id = 9000 + index as u64;
            let copy = spawn_source_copy(&mut h, id, difficulty);
            let herald = actors_in_copy(&mut h, 17307, copy)[0];
            let sentries = actors_in_copy(&mut h, 17517, copy);
            let point = *h.app.world().get::<Position>(sentries[0]).unwrap();
            let (x, y, z) = crate::terrain_height::bevy_to_wow(point.x, point.y, point.z);
            let (attacker, connection) = spawn_player(&mut h.app, copy, [x, y, z, 0.0], id);
            h.players.push((attacker, connection, Inbox::default()));
            reach_first_west(&mut h, copy, herald, &sentries);
            assert_eq!(native_avoidance_upper(&h, attacker, herald), avoid);
            pairs.push((copy, herald, attacker));
        }
        for (_, actor, attacker) in &pairs {
            reduce_to_one(&mut h, *actor);
            let position = *h.app.world().get::<Position>(*actor).unwrap();
            h.app.world_mut().entity_mut(*attacker).insert((
                Position {
                    x: position.x + 3.0,
                    ..position
                },
                crate::combat::AutoAttack::new(*actor),
            ));
        }
        // One real frame, native swing timers and stats. No retry clock can run
        // any actor to point4 arrival; each landed hit must commit to exactly1HP.
        h.advance(0.05);
        let mut landed = 0;
        for (copy, actor, attacker) in pairs {
            assert_alive_at_one(&mut h, actor, copy);
            assert_ne!(h.app.world().get::<Position>(actor), Some(&DESCEND));
            for (_, _, inbox) in &h.players {
                landed += inbox
                    .log
                    .iter()
                    .filter(|event| {
                        event.source == Some(attacker.to_bits())
                            && event.target == Some(actor.to_bits())
                            && event.spell_id.is_none()
                            && event.kind == CombatLogKind::Damage
                            && event.amount > 0
                    })
                    .count();
            }
        }
        assert!(
            landed > 0,
            "native no-hit probability is bounded below1/1000, never forced"
        );
    }
}
