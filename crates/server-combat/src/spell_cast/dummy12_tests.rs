//! Mainline native paths: actor bindings, successful spend and owned snapshots.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Npc, PowerType};
use shared::protocol::CombatLogKind;

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell: u32, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn actor_data(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(super::guardian::load_guardian_spells(&conn).unwrap());
}

fn actors(h: &mut Harness, entry: u32) -> Vec<Entity> {
    let world = h.app.world_mut();
    world
        .query::<(Entity, &Npc, &super::summon::SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == entry && summon.owner == h.caster)
        .map(|(unit, _, _)| unit)
        .collect()
}

fn damage(h: &Harness, spell: u32, target: Entity) -> Vec<i32> {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(spell)
                && e.target == Some(target.to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .map(|e| e.amount)
        .collect()
}

#[test]
fn dummy12_feral_spirit_summons_two_authentic_wolves_and_expires() {
    let mut h = harness(Vec::new(), &[51533, 228562, 58867, 58875, 58876], 7).unwrap();
    actor_data(&mut h);
    h.select(Some(h.npc));
    h.advance(4);
    h.cast_and_deliver(51533, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let wolves = actors(&mut h, 29264);
    assert_eq!(wolves.len(), 2);
    for wolf in &wolves {
        assert!(
            crate::auras::has_aura(h.app.world(), *wolf, 58875),
            "archive Spirit Walk cast"
        );
    }
    h.advance(400);
    assert!(
        wolves
            .iter()
            .all(|wolf| h.app.world().get_entity(*wolf).is_err())
    );
    assert_wolf_summon_events(&h, &wolves);
}

#[test]
fn native_feral_spirit_summon_carries_source_creature_flags_extra() {
    let mut h = harness(Vec::new(), &[51533, 228562, 58867, 58875, 58876], 7).unwrap();
    actor_data(&mut h);
    let expected = h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .templates[&29264]
        .flags_extra;
    h.cast_and_deliver(51533, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let wolves = actors(&mut h, 29264);
    assert_eq!(wolves.len(), 2);
    for wolf in wolves {
        assert_eq!(
            h.app
                .world()
                .get::<crate::zones::CreatureFlagsExtra>(wolf)
                .map(|flags| flags.0),
            Some(expected),
            "native Spirit Wolf spawn caches its authored template flags"
        );
    }
}

#[test]
fn class_completeness_native_feral_spirit_wolves_inherit_owner_ap_on_dummy_hits() {
    let mut h = harness(Vec::new(), &[51533, 228562, 58867, 58875, 58876], 7).unwrap();
    actor_data(&mut h);
    let caster = h.caster;
    // Native70391 child228562 selects65: 3yd behind the explicit target.
    // Summon behind the original NPC, then attack the dummy at that destination.
    // Casting at this dummy instead would move the summon center another3yd.
    let child = world_db().unwrap().store.get(228562).unwrap();
    let destination =
        super::targets::effect_dest(h.app.world(), caster, Some(h.npc), &child.effects[0], None)
            .unwrap();
    let dummy = crate::zones::summon_creature(
        h.app.world_mut(),
        caster,
        44548,
        [
            destination.position.x,
            -destination.position.z,
            destination.position.y,
            destination.orientation,
        ],
    )
    .expect("native Training Dummy template");
    assert!(
        h.app
            .world()
            .get::<crate::creature_scripts::TrainingDummy>(dummy)
            .is_some()
    );
    h.cast_and_deliver(51533, Some(h.npc));
    h.select(Some(dummy));
    h.attack(dummy);
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let wolves = actors(&mut h, 29264);
    assert_eq!(wolves.len(), 2);
    // Exercise the same native swing clock with shorter simulation frames.
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(10),
        ));
    h.inbox.log.clear();
    advance_wolf_swing_periods(&mut h, &wolves);
    for wolf in &wolves {
        assert_eq!(
            super::effects::power_stats(h.app.world(), *wolf, false),
            (31.0, 31.0)
        );
        assert_eq!(
            super::effects::power_stats(h.app.world(), *wolf, true),
            (31.0, 31.0)
        );
    }
    assert_eq!(
        super::effects::power_stats(h.app.world(), dummy, false),
        (0.0, 0.0)
    );
    let before: Vec<_> = wolves
        .iter()
        .map(|wolf| wolf_swing_attempts(&h, *wolf, dummy))
        .collect();
    let swing_diagnostics: Vec<_> = wolves
        .iter()
        .map(|wolf| {
            let position = h.app.world().get::<shared::components::Position>(*wolf);
            let attack = h.app.world().get::<crate::combat::AutoAttack>(*wolf);
            let events: Vec<_> = h
                .inbox
                .log
                .iter()
                .filter(|event| {
                    event.source == Some(wolf.to_bits()) && event.target == Some(dummy.to_bits())
                })
                .collect();
            format!("{wolf:?}: position={position:?}, attack={attack:?}, events={events:?}")
        })
        .collect();
    assert!(
        before.iter().all(|attempts| !attempts.is_empty()),
        "initial wolf attempts {before:?}; primary destination {destination:?}; dummy {:?}; fixed elapsed {:?}; wolves {swing_diagnostics:?}",
        h.app.world().get::<shared::components::Position>(dummy),
        h.app.world().resource::<Time<Fixed>>().elapsed()
    );
    for wolf in &wolves {
        assert_wolf_landed_damage(&h, *wolf, dummy, false);
    }
    h.inbox.log.clear();
    // Change only the real owner's melee AP, never actor stats or RNG. The
    // hundredfold input change must dominate ordinary block/crit variance.
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .attack_power *= 100.0;
    advance_wolf_swing_periods(&mut h, &wolves);
    assert_eq!(
        super::effects::power_stats(h.app.world(), dummy, false),
        (0.0, 0.0)
    );
    for wolf in wolves {
        assert_eq!(
            super::effects::power_stats(h.app.world(), wolf, false),
            (3100.0, 3100.0)
        );
        let attempts = wolf_swing_attempts(&h, wolf, dummy);
        assert!(!attempts.is_empty(), "wolf {wolf:?}: no AP-scaled attempts");
        assert_wolf_landed_damage(&h, wolf, dummy, true);
    }
}

/// Four native 1500ms swing periods per AP phase fit within the 15s summon.
/// Wait on the Fixed clock consumed by auto-attacks, regardless of frame size.
fn advance_wolf_swing_periods(h: &mut Harness, wolves: &[Entity]) {
    let swing_period = wolves
        .iter()
        .map(|wolf| {
            let wolf = h
                .app
                .world()
                .get::<super::guardian::SpiritWolf>(*wolf)
                .unwrap();
            std::time::Duration::from_secs_f32(wolf.weapon.speed)
        })
        .max()
        .unwrap();
    let deadline = h.app.world().resource::<Time<Fixed>>().elapsed() + swing_period * 4;
    while h.app.world().resource::<Time<Fixed>>().elapsed() < deadline {
        h.advance(1);
    }
}

fn wolf_swing_attempts(
    h: &Harness,
    wolf: Entity,
    dummy: Entity,
) -> Vec<&shared::protocol::CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.source == Some(wolf.to_bits())
                && event.target == Some(dummy.to_bits())
                && event.spell_id.is_none()
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .collect()
}

fn assert_wolf_landed_damage(h: &Harness, wolf: Entity, dummy: Entity, increased_ap: bool) {
    let weapon = &h
        .app
        .world()
        .get::<super::guardian::SpiritWolf>(wolf)
        .unwrap()
        .weapon;
    // Native owner contribution31 at100AP. Even a baseline critical maximum
    // must be dwarfed by a landed hit after the hundredfold owner AP change.
    let baseline_critical_max = ((weapon.max + 31.0 / 3.5 * weapon.speed).round() * 2.0) as i32;
    for event in wolf_swing_attempts(h, wolf, dummy) {
        if event.kind != CombatLogKind::Damage {
            continue;
        }
        let damage_before_block = event.amount + event.blocked;
        if increased_ap {
            assert!(
                damage_before_block > baseline_critical_max * 10,
                "{event:?}"
            );
        } else {
            assert!(
                (1..=baseline_critical_max).contains(&damage_before_block),
                "{event:?}"
            );
        }
    }
}

#[test]
fn dummy12_feral_spirit_child_logs_two_short_lived_wolves_to_owner() {
    let mut h = harness(Vec::new(), &[51533, 228562, 58867, 58875, 58876], 7).unwrap();
    actor_data(&mut h);
    h.select(Some(h.npc));
    h.advance(4);
    let caster = h.caster;
    let target = h.npc;
    super::cast_triggered(h.app.world_mut(), caster, 228562, Some(target), 0);
    h.advance(3);
    let wolves = actors(&mut h, 29264);
    assert_eq!(
        wolves.len(),
        2,
        "one native child casts its authored two wolves"
    );
    h.advance(400);
    assert!(
        wolves
            .iter()
            .all(|wolf| h.app.world().get_entity(*wolf).is_err())
    );
    assert_wolf_summon_events(&h, &wolves);
}

fn assert_wolf_summon_events(h: &Harness, wolves: &[Entity]) {
    let summons: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Summon)
        .collect();
    assert_eq!(
        summons.len(),
        2,
        "short-lived wolves need durable owner events"
    );
    for wolf in wolves {
        let event = summons
            .iter()
            .find(|event| event.target == Some(wolf.to_bits()))
            .expect("one event per actual wolf");
        assert_eq!(event.source, Some(h.caster.to_bits()));
        assert_eq!(event.spell_id, Some(228562), "actual child summoning spell");
        assert!(event.timestamp_unix_ms > 0);
    }
}

#[test]
fn dummy12_guardian_damage_reaches_owner_without_guardian_connection() {
    let mut h = harness(Vec::new(), &[49206, 51963, 61777, 1242147, 211947], 6).unwrap();
    actor_data(&mut h);
    h.select(Some(h.npc));
    h.advance(4);
    h.cast_and_deliver(49206, Some(h.npc));
    let gargoyle = actors(&mut h, 27829)[0];
    // Isolate delivery from the unimplemented non-demon AP inheritance curve,
    // using the same explicit actor stats as the existing Gargoyle script fixture.
    h.app
        .world_mut()
        .entity_mut(gargoyle)
        .insert(crate::stats::CharacterStats {
            attack_power: 100.0,
            ..default()
        });
    h.advance(100);
    let summon_index = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.kind == CombatLogKind::Summon && event.target == Some(gargoyle.to_bits())
        })
        .expect("owner receives gargoyle summon");
    let damage_index = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(gargoyle.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.amount > 0
        })
        .unwrap_or_else(|| {
            panic!(
                "owner receives actual guardian damage without a guardian client: {:?}",
                h.inbox.log
            )
        });
    assert!(
        summon_index < damage_index,
        "ownership arrives before guardian damage"
    );
}

#[test]
fn dummy12_gargoyle_native_attack_crit_and_successful_owner_spend() {
    let mut h = harness(
        Vec::new(),
        &[49206, 51963, 61777, 1242147, 211947, 47541, 47632, 77575],
        6,
    )
    .unwrap();
    actor_data(&mut h);
    h.select(Some(h.npc));
    h.advance(4);
    h.cast_and_deliver(49206, Some(h.npc));
    let gargoyle = actors(&mut h, 27829)[0];
    // Explicit actor stats isolate this script; non-demon stat inheritance is not claimed.
    h.app
        .world_mut()
        .entity_mut(gargoyle)
        .insert(crate::stats::CharacterStats {
            attack_power: 100.0,
            ..default()
        });
    let observer = spawn_peer_connection(&mut h.app, gargoyle, 2);
    h.set_power(PowerType::RunicPower, 1000.0);
    let spell = world_db().unwrap().store.get(47541).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
    assert_eq!(h.power(PowerType::RunicPower), 700.0);
    advance_actor_logs(&mut h, observer, 100);
    let bolts: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(51963) && e.kind == CombatLogKind::Damage)
        .collect();
    assert!(!bolts.is_empty());
    assert!(
        bolts.iter().all(|e| e.crit),
        "authored always-critical attack"
    );
    assert!(
        bolts
            .iter()
            .any(|e| varied(34.776 * 2.0 * 1.3).contains(&(e.amount as f32))),
        "30 spent displayed RP =>30% bonus: {bolts:?}"
    );
    // A refused cast cannot increase the guardian's bonus.
    h.set_power(PowerType::RunicPower, 0.0);
    assert!(super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).is_err());
    h.inbox.log.clear();
    advance_actor_logs(&mut h, observer, 80);
    assert!(
        !damage(&h, 51963, h.npc).is_empty(),
        "real guardian attacks must continue after refused owner cast"
    );
    assert!(
        damage(&h, 51963, h.npc)
            .iter()
            .all(|&n| varied(34.776 * 2.0 * 1.3).contains(&(n as f32)))
    );
}

fn advance_actor_logs(h: &mut Harness, connection: Entity, frames: usize) {
    for _ in 0..frames {
        h.advance(1);
        h.inbox
            .log
            .extend(drain::<shared::protocol::CombatLogEvent>(
                &mut h.app, connection,
            ));
    }
}

#[test]
fn dummy12_dispatch_and_eviscerate_damage_use_combo_spent_not_left() {
    for spell_id in [2098, 196819] {
        for points in [1.0, 3.0, 5.0] {
            let mut h =
                harness_with_native_enemy_dodge_reduction(Vec::new(), &[spell_id], 4).unwrap();
            h.set_power(PowerType::ComboPoints, points);
            h.set_power(PowerType::Energy, 100.0);
            let spell = world_db().unwrap().store.get(spell_id).unwrap();
            let base =
                super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
            let before = h.health(h.npc);
            super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
            assert_eq!(h.power(PowerType::ComboPoints), 0.0);
            assert!(
                varied(base * points).contains(&(before - h.health(h.npc))),
                "spell={spell_id}, points={points}, loss={}",
                before - h.health(h.npc)
            );
        }
    }
}

#[test]
fn dummy12_rake_bleed_snapshots_stealth_bonus_across_later_rank_removal() {
    for empowered in [false, true] {
        let mut h = harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[1822, 155722, 231052, 5215, 768, 163505],
            11,
        )
        .unwrap();
        let caster = h.caster;
        apply(&mut h, caster, caster, 768, u32::MAX);
        apply(&mut h, caster, caster, 231052, u32::MAX);
        if empowered {
            apply(&mut h, caster, caster, 5215, 1);
        }
        let before = h.health(h.npc);
        super::execute_cast(
            h.app.world_mut(),
            caster,
            world_db().unwrap().store.get(1822).unwrap(),
            Some(h.npc),
        )
        .unwrap();
        assert!(h.health(h.npc) < before);
        // Effect64 applies the native zero-delay child on the next event update.
        h.advance(1);
        assert!(crate::auras::has_aura(h.app.world(), h.npc, 155722));
        crate::auras::remove_auras(
            h.app.world_mut(),
            caster,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == 5215 || a.spell_id == 231052,
        );
        h.advance(135);
        let ticks = damage(&h, 155722, h.npc);
        assert_eq!(ticks.len(), 2);
        assert!(
            ticks
                .iter()
                .all(|&n| varied(29.4 * if empowered { 1.6 } else { 1.0 }).contains(&(n as f32))),
            "{ticks:?}"
        );
    }
}

#[test]
fn dummy12_unstable_affliction_vulnerability_is_owned_not_stacked_and_expires() {
    let mut h = harness(Vec::new(), &[30108, 233490, 108853], 9).unwrap();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, caster, target, 233490, 1);
    apply(&mut h, caster, target, 233490, 1);
    let spell = world_db().unwrap().store.get(108853).unwrap();
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), caster, 108853, Some(h.npc), 0);
    assert!(varied(197.25 * 1.1).contains(&(before - h.health(h.npc))));
    let foreign = spawn_player(&mut h.app, 9, 0.0);
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), foreign, spell.id, Some(h.npc), 0);
    assert!(varied(197.25).contains(&(before - h.health(h.npc))));
    h.advance(350);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 233490));
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), caster, spell.id, Some(h.npc), 0);
    assert!(varied(197.25).contains(&(before - h.health(h.npc))));
}

#[test]
fn dummy12_shield_of_righteous_applies_native_armor_buff_and_expires() {
    let mut h = harness(Vec::new(), &[53600, 132403, 403460, 407467], 2).unwrap();
    h.set_power(PowerType::HolyPower, 3.0);
    let caster = h.caster;
    h.cast_and_deliver(53600, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), caster, 132403));
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(caster)
            .unwrap()
            .armor_from_primary_stat_pct,
        160.0
    );
    assert_eq!(h.power(PowerType::HolyPower), 0.0);
    h.advance(110);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 132403));
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(caster)
            .unwrap()
            .armor_from_primary_stat_pct,
        0.0
    );
}

#[test]
fn dummy12_shield_cone_excludes_rear_side_far_and_ally_and_buffs_empty_cast() {
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[53600, 132403, 403460, 407467], 2)
            .unwrap();
    let rear = spawn_npc(&mut h.app, -3.0);
    let side = spawn_npc(&mut h.app, 2.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(side)
        .unwrap()
        .z = 3.5;
    let edge = spawn_npc(&mut h.app, 3.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(edge)
        .unwrap()
        .z = 5.0;
    let far = spawn_npc(&mut h.app, 7.0);
    let ally = spawn_player(&mut h.app, 2, 3.0);
    h.set_power(PowerType::HolyPower, 3.0);
    let spell = world_db().unwrap().store.get(53600).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, None).unwrap();
    for target in [h.npc, edge] {
        assert!(varied(95.0).contains(&(5000.0 - h.health(target))));
    }
    for target in [rear, side, far] {
        assert_eq!(h.health(target), 5000.0);
    }
    assert_eq!(h.health(ally), 1000.0);
    h.advance(110);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .x = 50.0;
    h.set_power(PowerType::HolyPower, 3.0);
    super::execute_cast(h.app.world_mut(), h.caster, spell, None).unwrap();
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 132403));
}

#[test]
fn dummy12_rake_reapplication_replaces_stealth_snapshot_without_retroactive_bonus() {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[1822, 155722, 231052, 5215, 768, 163505],
        11,
    )
    .unwrap();
    let caster = h.caster;
    apply(&mut h, caster, caster, 768, u32::MAX);
    apply(&mut h, caster, caster, 231052, u32::MAX);
    apply(&mut h, caster, caster, 5215, 1);
    let root = world_db().unwrap().store.get(1822).unwrap();
    super::execute_cast(h.app.world_mut(), caster, root, Some(h.npc)).unwrap();
    h.advance(65);
    assert!(
        damage(&h, 155722, h.npc)
            .iter()
            .all(|&n| varied(29.4 * 1.6).contains(&(n as f32)))
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 5215,
    );
    h.inbox.log.clear();
    h.set_power(PowerType::Energy, 100.0);
    super::execute_cast(h.app.world_mut(), caster, root, Some(h.npc)).unwrap();
    h.advance(65);
    assert_eq!(damage(&h, 155722, h.npc), vec![29]);
    apply(&mut h, caster, caster, 5215, 1);
    h.inbox.log.clear();
    h.advance(65);
    assert_eq!(damage(&h, 155722, h.npc), vec![29]);
}

#[test]
fn dummy12_gargoyle_foreign_owner_spend_and_triggered_cast_do_not_empower() {
    let ids = [49206, 51963, 61777, 1242147, 211947, 47541, 47632, 77575];
    let mut h = harness(Vec::new(), &ids, 6).unwrap();
    actor_data(&mut h);
    h.select(Some(h.npc));
    h.advance(4);
    h.cast_and_deliver(49206, Some(h.npc));
    let actor = actors(&mut h, 27829)[0];
    h.app
        .world_mut()
        .entity_mut(actor)
        .insert(crate::stats::CharacterStats {
            attack_power: 100.0,
            ..default()
        });
    let observer = spawn_peer_connection(&mut h.app, actor, 2);
    let foreign = spawn_player(&mut h.app, 6, 0.0);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(super::powers::PowerPools::from_table(
            &power_table(world_db().unwrap()),
            vec![PowerType::RunicPower],
        ));
    super::powers::change_power(h.app.world_mut(), foreign, PowerType::RunicPower, 1000.0);
    apply(&mut h, foreign, foreign, 1242147, u32::MAX);
    super::execute_cast(
        h.app.world_mut(),
        foreign,
        world_db().unwrap().store.get(47541).unwrap(),
        Some(h.npc),
    )
    .unwrap();
    super::cast_triggered(h.app.world_mut(), h.caster, 47541, Some(h.npc), 1);
    assert!(!crate::auras::has_aura(h.app.world(), actor, 211947));
    advance_actor_logs(&mut h, observer, 120);
    assert!(!damage(&h, 51963, h.npc).is_empty());
    assert!(
        damage(&h, 51963, h.npc)
            .iter()
            .all(|&n| varied(34.776 * 2.0).contains(&(n as f32)))
    );
    h.advance(600);
    assert!(h.app.world().get_entity(actor).is_err());
}

#[test]
fn dummy12_butchery_talent_bleeds_hit_enemies_only() {
    for talented in [false, true] {
        let mut h =
            harness_with_native_enemy_dodge_reduction(Vec::new(), &[212436, 459868, 459870], 3)
                .unwrap();
        if !talented {
            h.app.world_mut().entity_mut(h.caster).insert(
                crate::class_progression::KnownSpells::fixture(vec![212436, 459870]),
            );
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Cancelled,
                |a| a.spell_id == 459868,
            );
        }
        let near = spawn_npc(&mut h.app, 4.0);
        let far = spawn_npc(&mut h.app, 30.0);
        let ally = spawn_player(&mut h.app, 3, 2.0);
        h.set_power(PowerType::Focus, 100.0);
        h.cast_and_deliver(212436, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        for target in [h.npc, near] {
            assert_eq!(
                crate::auras::has_aura(h.app.world(), target, 459870),
                talented
            );
        }
        for target in [far, ally] {
            assert!(!crate::auras::has_aura(h.app.world(), target, 459870));
        }
        h.advance(85);
        assert_eq!(damage(&h, 459870, near).is_empty(), !talented);
        h.advance(300);
        assert!(!crate::auras::has_aura(h.app.world(), near, 459870));
    }
}
