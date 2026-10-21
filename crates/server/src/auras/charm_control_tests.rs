//! Native spells through authenticated pet/movement messages, damage and removal.
use super::{AuraApplication, AuraSource, RemoveReason, apply_aura, has_aura, remove_auras};
use crate::spell_cast::test_harness::{
    Harness, harness_with_plugins, load_native_creature_world, spawn_npc,
};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{
    MovementControl, MovementSpeed, PlayerMotion, Position, Rotation, UnitCharm,
    UnitFactionTemplate, VerticalVelocity,
};
use shared::protocol::{
    ACT_COMMAND, COMMAND_ABANDON, COMMAND_ATTACK, CombatChannel, InputChannel, PetAction,
    PetClearSpells, PetSpells, PlayerInput, pet_action_button,
};

fn fixture() -> Harness {
    fixture_for(5)
}

fn fixture_for(class: u8) -> Harness {
    let mut h = harness_with_plugins(
        Vec::new(),
        &[605, 1098, 205364, 31865, 133, 3110],
        class,
        false,
        |app| {
            app.insert_resource(load_native_creature_world(&[6, 416]));
            app.add_plugins((
                crate::pet_control::PetControlPlugin,
                crate::creature_ai::CreatureAiPlugin,
            ));
            app.init_resource::<crate::friends::AutoAfkScheduler>();
            app.add_systems(FixedUpdate, crate::networking::process_player_inputs);
        },
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<PetAction>::default(),
        MessageReceiver::<PetAction>::default(),
        MessageSender::<PetSpells>::default(),
        MessageReceiver::<PetSpells>::default(),
        MessageSender::<PetClearSpells>::default(),
        MessageReceiver::<PetClearSpells>::default(),
        MessageSender::<PlayerInput>::default(),
        MessageReceiver::<PlayerInput>::default(),
    ));
    h.app.world_mut().entity_mut(h.caster).insert((
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        MovementSpeed(0.0),
        PlayerMotion::default(),
        VerticalVelocity(0.0),
        crate::networking::FallTracker::grounded(0.0),
    ));
    // Production zones::spawn_creature publishes a stationary NPC speed of zero.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(MovementSpeed(0.0));
    h
}

fn apply(h: &mut Harness, spell: u32) {
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn order(h: &mut Harness, command: u32, target: Option<Entity>) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<PetAction>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(PetAction {
            pet: h.npc.to_bits(),
            action: pet_action_button(command, ACT_COMMAND),
            target: target.map(Entity::to_bits),
            position: None,
        });
    h.advance(4);
}

#[test]
#[ignore = "RED: possess movement authority needs native message proof"]
fn charm_control_native_605_routes_input_to_npc_not_priest_and_rejects_old_epoch() {
    let mut h = fixture();
    let original = *h.app.world().get::<Position>(h.caster).unwrap();
    apply(&mut h, 605);
    let control = *h.app.world().get::<MovementControl>(h.npc).unwrap();
    assert!(
        h.app.world().get::<MovementSpeed>(h.npc).unwrap().0 > 0.0,
        "taking possession must publish movement speed, not the NPC's prior idle speed0"
    );
    assert_eq!(
        *h.app.world().get::<UnitCharm>(h.npc).unwrap(),
        UnitCharm {
            charmer: h.caster.to_bits(),
            possessed: true
        }
    );
    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
        flying: false,
        position: [4.0, 0.0, 0.0],
        epoch: control.epoch,
        transport: None,
    };
    h.app
        .world_mut()
        .get_mut::<MessageSender<PlayerInput>>(h.connection)
        .unwrap()
        .send::<InputChannel>(input.clone());
    h.advance(5);
    let moved = *h.app.world().get::<Position>(h.npc).unwrap();
    assert!(
        moved.x > 3.0 && moved.x <= 4.001,
        "authoritative possessed mover: {moved:?}"
    );
    assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), original);
    h.app
        .world_mut()
        .get_mut::<MessageSender<PlayerInput>>(h.connection)
        .unwrap()
        .send::<InputChannel>(PlayerInput {
            epoch: control.epoch.wrapping_sub(1),
            position: [20.0, 0.0, 0.0],
            ..input
        });
    h.advance(5);
    assert_eq!(
        *h.app.world().get::<Position>(h.npc).unwrap(),
        moved,
        "stale body input cannot move possessed unit"
    );
    order(&mut h, COMMAND_ABANDON, None);
    assert!(!has_aura(h.app.world(), h.npc, 605));
    assert!(h.app.world().get::<UnitCharm>(h.npc).is_none());
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    assert!(
        h.app.world().get_entity(h.npc).is_ok(),
        "release is not pet deletion"
    );
}

#[test]
#[ignore = "RED: native charm action and actual melee consequence unproven"]
fn charm_control_native_205364_pet_attack_hits_enemy_not_charmer() {
    let mut h = fixture();
    let enemy = spawn_npc(&mut h.app, 4.0);
    let before = h.health(enemy);
    let owner_before = h.health(h.caster);
    apply(&mut h, 205364);
    order(&mut h, COMMAND_ATTACK, Some(enemy));
    h.advance(120);
    assert!(
        h.health(enemy) < before,
        "dominated NPC must actually damage the ordered enemy"
    );
    assert_eq!(
        h.health(h.caster),
        owner_before,
        "original hostile AI cannot attack its charmer"
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(h.npc)
            .unwrap()
            .victim,
        enemy
    );
    order(&mut h, COMMAND_ABANDON, None);
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
        25
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(h.npc)
            .is_none()
    );
}

#[test]
#[ignore = "RED: native damage break and restoration unproven"]
fn charm_control_native_205364_retains_damage_but_31865_breaks_from_native_flags() {
    for (spell, retained) in [(205364, true), (31865, false)] {
        let mut h = fixture();
        let attacker = spawn_npc(&mut h.app, 4.0);
        apply(&mut h, spell);
        let damage = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(133)
            .unwrap()
            .clone();
        crate::spell_cast::effects::deal_damage(
            h.app.world_mut(),
            &crate::spell_cast::effects::EffectCtx {
                caster: attacker,
                spell: &damage,
                depth: 0,
                dest: None,
                chain_multiplier: 1.0,
            },
            h.npc,
            50.0,
            false,
            false,
        );
        h.advance(4);
        assert_eq!(
            has_aura(h.app.world(), h.npc, spell),
            retained,
            "spell{spell} native interrupt flags"
        );
        assert_eq!(h.app.world().get::<UnitCharm>(h.npc).is_some(), retained);
        if retained {
            remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |a| {
                a.spell_id == spell
            });
        }
        assert_eq!(
            h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
            25
        );
    }
}

#[test]
#[ignore = "RED: TDB native Imp spell3110 is not on the charm bar"]
fn charm_control_native_1098_uses_authentic_imp_tdb_spell_slots() {
    let mut h = fixture_for(9);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::Npc {
            template_id: 416,
            name: "Imp".into(),
        });
    //The spell's real source row is CreatureID416/Index0,2/Spell3110.
    // Native Subjugate's demon restriction must be tested on an actual demon identity.
    for _ in 0..3 {
        h.cast(1098, Some(h.npc));
        h.advance(100);
        if has_aura(h.app.world(), h.npc, 1098) {
            break;
        }
    }
    assert!(
        has_aura(h.app.world(), h.npc, 1098),
        "native warlock1098 casts onto source Imp416: {:?}",
        h.inbox.failed
    );
    let bar = &h
        .app
        .world()
        .get::<crate::pet_control::PetCharm>(h.npc)
        .unwrap()
        .action_bar;
    let spells: Vec<_> = bar
        .iter()
        .filter(|button| shared::protocol::pet_action_button_action(**button) == 3110)
        .collect();
    assert_eq!(
        spells.len(),
        1,
        "native duplicated source3110 creates one Firebolt action"
    );
}
