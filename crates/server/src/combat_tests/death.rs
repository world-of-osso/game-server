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
