use super::*;
use crate::death::{DeathPlugin, PlayerDeathState};
use crate::durability::{DurabilitySlotState, EquippedDurability};
use crate::persistence::ClientId;
use shared::components::EquipmentVisualSlot;
use shared::death::DeathState as PlayerDeathKind;

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
            AutoAttack {
                timer: 0.0,
                active: true,
            },
            Player {
                name: "Attacker".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
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
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatTarget(Some(h.npc)));
    h.advance(5);

    let (caster, npc) = (Some(h.caster.to_bits()), Some(h.npc.to_bits()));
    assert_eq!(
        logged(&h),
        vec![
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
        AutoAttack {
            timer: 0.0,
            active: true,
        },
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
