use super::*;
use crate::death::{DeathPlugin, PlayerDeathState};
use crate::durability::{DurabilitySlotState, EquippedDurability};
use crate::persistence::ClientId;
use shared::components::EquipmentVisualSlot;
use shared::death::DeathState as PlayerDeathKind;

/// MOD_HIT_CHANCE 100: a creature that never misses (Unit::MeleeSpellMissChance), so a
/// fixture's lethal swing lands; it may still crit.
fn never_misses() -> crate::auras::AuraStatMods {
    crate::auras::AuraStatMods {
        melee_hit_chance_pct: 100.0,
        ..Default::default()
    }
}

#[test]
fn lethal_player_auto_attack_transitions_death_in_same_fixed_step() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(DeathPlugin);
    app.add_plugins(CombatPlugin);
    app.init_resource::<FactionTemplates>();
    app.finish();
    app.cleanup();

    let target = app
        .world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            Health {
                current: 5.0,
                max: 100.0,
            },
            crate::maps::InMap::continent(0),
            Player {
                name: "Target".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            ClientId(2),
            EquippedDurability {
                slots: vec![DurabilitySlotState {
                    slot: EquipmentVisualSlot::Head,
                    item_id: 0,
                    current: 70,
                    max: 70,
                }],
            },
        ))
        .id();
    let attacker = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            CombatTarget(Some(target)),
            AutoAttack::new(target),
            Npc {
                template_id: 6,
                name: "Kobold Vermin".into(),
            },
            never_misses(),
        ))
        .id();

    app.world_mut().run_schedule(FixedUpdate);

    assert_eq!(
        app.world().entity(target).get::<Health>().unwrap().current,
        0.0,
        "combat must clamp lethal player health to zero"
    );
    assert_eq!(
        app.world().entity(target).get::<LastDamager>().unwrap().0,
        attacker,
        "combat must preserve the attacker as last damager"
    );
    assert_eq!(
        app.world()
            .entity(target)
            .get::<PlayerDeathState>()
            .expect("lethal combat must transition the player in the same fixed step")
            .state,
        PlayerDeathKind::Dead
    );
    assert_eq!(
        app.world()
            .entity(target)
            .get::<EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        63,
        "death durability loss must apply once"
    );

    app.world_mut().run_schedule(FixedUpdate);

    assert_eq!(
        app.world()
            .entity(target)
            .get::<PlayerDeathState>()
            .unwrap()
            .state,
        PlayerDeathKind::Dead
    );
    assert_eq!(
        app.world()
            .entity(target)
            .get::<EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        63,
        "a second fixed step must not repeat the death transition"
    );
}

/// AzerothCore `Player::setDeathState` (Player.cpp:1078) calls
/// `Unit::setDeathState`, whose `CombatStop` (Unit.cpp:11439-11441) runs for
/// players too: the dying player stops attacking and its attackers stop
/// attacking it, so a player resurrected in melee range is not swung at, and a
/// dead player's auto-attack does not keep hitting its target.
#[test]
fn a_player_resurrected_in_melee_range_is_not_attacked_by_its_killer() {
    use bevy::time::TimeUpdateStrategy;
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(DeathPlugin);
    app.add_plugins(CombatPlugin);
    app.init_resource::<FactionTemplates>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.finish();
    app.cleanup();

    let player = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            Health {
                current: BASE_DAMAGE * 3.0,
                max: 100.0,
            },
            crate::maps::InMap::continent(0),
            Player {
                name: "Target".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            ClientId(2),
        ))
        .id();
    let kobold = app
        .world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            Health {
                current: 100.0,
                max: 100.0,
            },
            Npc {
                template_id: 6,
                name: "Kobold Vermin".into(),
            },
            CombatTarget(Some(player)),
            never_misses(),
        ))
        .id();
    app.world_mut()
        .entity_mut(player)
        .insert(CombatTarget(Some(kobold)));

    // Three kobold swings, 2 s apart.
    for _ in 0..(AUTO_ATTACK_INTERVAL * 2.0 * 20.0) as usize + 10 {
        app.update();
    }
    let dead = app.world().get::<PlayerDeathState>(player).expect("killed");
    assert_eq!(dead.state, PlayerDeathKind::Dead);
    let kobold_health = app.world().get::<Health>(kobold).unwrap().current;

    // Resurrected where it died, in the kobold's melee range.
    app.world_mut()
        .get_mut::<PlayerDeathState>(player)
        .unwrap()
        .state = PlayerDeathKind::Alive;
    app.world_mut().get_mut::<Health>(player).unwrap().current = 50.0;
    for _ in 0..(AUTO_ATTACK_INTERVAL * 2.0 * 20.0) as usize {
        app.update();
    }

    assert_eq!(app.world().get::<Health>(player).unwrap().current, 50.0);
    assert!(app.world().get::<AutoAttack>(kobold).is_none());
    assert!(app.world().get::<AutoAttack>(player).is_none());
    assert_eq!(
        app.world().get::<Health>(kobold).unwrap().current,
        kobold_health,
        "the player's auto-attack stopped when it died"
    );
}

/// (kind, source, target) of the combat log events the harness caster received.
fn logged(
    h: &crate::spell_cast::test_harness::Harness,
) -> Vec<(CombatLogKind, Option<u64>, Option<u64>)> {
    h.inbox
        .log
        .iter()
        .map(|event| (event.kind, event.source, event.target))
        .collect()
}

/// Unit::DealDamage → Unit::Kill for melee as for spells: the killing
/// swing's Damage line is followed by a Death line.
#[test]
fn lethal_melee_swing_on_an_npc_logs_death_after_the_damage() {
    let Some(mut h) = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1) else {
        return;
    };
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 5.0,
        max: 5.0,
    });
    let npc = h.npc;
    h.attack(npc);
    // Ten 2 s swings: the first that is not missed, dodged or parried kills.
    h.advance(5 + 9 * 40);

    let (caster, npc) = (Some(h.caster.to_bits()), Some(h.npc.to_bits()));
    let log = logged(&h);
    let avoided = log.len() - 2;
    assert!(
        log[..avoided].iter().all(
            |&(kind, source, target)| matches!(kind, CombatLogKind::Miss(_))
                && (source, target) == (caster, npc)
        ),
        "{log:?}"
    );
    assert_eq!(
        log[avoided..],
        [
            (CombatLogKind::Damage, caster, npc),
            (CombatLogKind::Death, caster, npc),
        ]
    );
    assert!(h.app.world().get::<DeathState>(h.npc).is_some());
}

#[test]
fn lethal_melee_swing_on_a_player_logs_death_to_the_victim() {
    let Some(mut h) = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1) else {
        return;
    };
    h.app.world_mut().entity_mut(h.caster).insert(Health {
        current: 5.0,
        max: 100.0,
    });
    let (caster, npc) = (h.caster, h.npc);
    h.app.world_mut().entity_mut(npc).insert((
        CombatTarget(Some(caster)),
        AutoAttack::new(caster),
        never_misses(),
    ));
    h.advance(5);

    let (caster, npc) = (Some(caster.to_bits()), Some(npc.to_bits()));
    assert_eq!(
        logged(&h),
        vec![
            (CombatLogKind::Damage, npc, caster),
            (CombatLogKind::Death, npc, caster),
        ]
    );
}
