use super::test_harness::*;
use crate::death::{DeathPlugin, PlayerDeathState};
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;
use shared::components::{Health, Mana, MovementControl, Position, UnitTap};
use shared::death::DeathState;
use shared::protocol::{ResurrectionOffer, ResurrectionResponse};

#[derive(Resource)]
struct FirstResurrection {
    target: Entity,
    restored: Option<(f32, f32, Position)>,
}

fn capture_first_resurrection(
    mut proof: ResMut<FirstResurrection>,
    players: Query<(&PlayerDeathState, &Health, &Mana, &Position)>,
) {
    if proof.restored.is_some() {
        return;
    }
    let (state, health, mana, position) = players.get(proof.target).unwrap();
    if state.state == DeathState::Alive {
        proof.restored = Some((health.current, mana.current, *position));
    }
}

fn load_native_redemption() -> SpellInfo {
    world_db()
        .expect("world fixture")
        .store
        .get(7328)
        .expect("native Redemption with corpse-target metadata")
        .clone()
}

fn fixture() -> (Harness, Entity, Entity) {
    let spell = load_native_redemption();
    let mut h = harness_with_plugins(vec![spell], &[], 2, false, |app| {
        app.add_plugins(DeathPlugin);
    })
    .expect("world fixture required");
    // The common combat fixture keeps rage from decaying; native Redemption is out-of-combat.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::CombatStatus(false));
    let target = spawn_player(&mut h.app, 5, 3.0);
    h.app.world_mut().entity_mut(target).insert((
        PlayerDeathState {
            state: DeathState::Dead,
            ..default()
        },
        MovementControl::default(),
        Health {
            current: 0.0,
            max: 1000.0,
        },
        Mana {
            current: 0.0,
            max: 2000.0,
        },
    ));
    h.app.insert_resource(FirstResurrection {
        target,
        restored: None,
    });
    h.app.add_systems(
        Update,
        capture_first_resurrection.after(crate::death::resurrection::receive_responses),
    );
    let conn = spawn_peer_connection(&mut h.app, target, 99);
    h.app.world_mut().entity_mut(conn).insert((
        lightyear::prelude::MessageSender::<shared::protocol::DeathStateUpdate>::default(),
        lightyear::prelude::MessageReceiver::<shared::protocol::DeathStateUpdate>::default(),
        lightyear::prelude::MessageSender::<ResurrectionOffer>::default(),
        lightyear::prelude::MessageReceiver::<ResurrectionOffer>::default(),
        lightyear::prelude::MessageSender::<ResurrectionResponse>::default(),
        lightyear::prelude::MessageReceiver::<ResurrectionResponse>::default(),
    ));
    (h, target, conn)
}

fn offer(h: &mut Harness, target: Entity, conn: Entity) -> ResurrectionOffer {
    offer_for(h, target, conn, 7328)
}

fn respond(h: &mut Harness, conn: Entity, response: ResurrectionResponse) {
    h.app
        .world_mut()
        .get_mut::<lightyear::prelude::MessageSender<ResurrectionResponse>>(conn)
        .unwrap()
        .send::<shared::protocol::DeathChannel>(response);
    h.advance(3);
}

#[test]
fn rezrtap_retail_resurrection_spells_offer_to_ghost() {
    for spell_id in [7328, 2006, 50769] {
        let (mut h, target, conn) = fixture();
        let spell = world_db()
            .expect("world fixture")
            .store
            .get(spell_id)
            .expect("Retail resurrection")
            .clone();
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![spell]));
        h.app.world_mut().entity_mut(h.caster).insert((
            crate::class_progression::KnownSpells::fixture(vec![spell_id]),
            shared::components::CombatStatus(false),
        ));
        h.app
            .world_mut()
            .get_mut::<PlayerDeathState>(target)
            .unwrap()
            .state = DeathState::Ghost;
        let offered = offer_for(&mut h, target, conn, spell_id);
        assert_eq!(offered.spell_id, spell_id);
        assert_eq!(h.health(target), 0.0);
    }
}

fn offer_for(h: &mut Harness, target: Entity, conn: Entity, spell_id: u32) -> ResurrectionOffer {
    h.cast(spell_id, Some(target));
    let mut offers = Vec::new();
    for _ in 0..250 {
        h.advance(1);
        offers.extend(drain::<ResurrectionOffer>(&mut h.app, conn));
    }
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(offers.len(), 1);
    offers.remove(0)
}

#[test]
fn rezrtap_cast_offer_accept_percent_and_caster_position() {
    let (mut h, target, conn) = fixture();
    let offer = offer(&mut h, target, conn);
    assert_eq!(offer.caster, h.caster.to_bits());
    assert_eq!(offer.caster_name, "Caster");
    assert_eq!(offer.spell_id, 7328);
    assert_eq!(offer.time_left_ms, 60_000);
    assert_eq!(h.health(target), 0.0);
    respond(
        &mut h,
        conn,
        ResurrectionResponse {
            caster: offer.caster,
            spell_id: 7328,
            accept: true,
        },
    );
    assert_eq!(
        h.app.world().get::<PlayerDeathState>(target).unwrap().state,
        DeathState::Alive
    );
    assert_eq!(h.health(target), 350.0);
    // Record the actual acceptance boundary, before subsequent gameplay regen ticks.
    let restored = h
        .app
        .world()
        .resource::<FirstResurrection>()
        .restored
        .unwrap();
    assert_eq!(restored.0, 350.0);
    assert_eq!(restored.1, 700.0);
    assert_eq!(Some(&restored.2), h.app.world().get::<Position>(h.caster));
    assert_eq!(
        h.app.world().get::<Position>(target),
        h.app.world().get::<Position>(h.caster)
    );
}

#[test]
fn rezrtap_decline_and_timeout_remain_dead() {
    for timeout in [false, true] {
        let (mut h, target, conn) = fixture();
        let offer = offer(&mut h, target, conn);
        if timeout {
            h.app
                .world_mut()
                .get_mut::<crate::death::resurrection::PendingResurrection>(target)
                .unwrap()
                .expires_at = -1.0;
        }
        respond(
            &mut h,
            conn,
            ResurrectionResponse {
                caster: offer.caster,
                spell_id: 7328,
                accept: timeout,
            },
        );
        assert_eq!(h.health(target), 0.0);
        assert_eq!(
            h.app.world().get::<PlayerDeathState>(target).unwrap().state,
            DeathState::Dead
        );
    }
}

#[test]
fn rezrtap_resurrect_with_aura_applies_trigger_after_acceptance() {
    let (mut h, target, conn) = fixture();
    let aura = world_db()
        .expect("world fixture")
        .store
        .get(15007)
        .expect("resurrection sickness fixture")
        .clone();
    let mut spell = load_native_redemption();
    // Synthetic effect172 acceptance boundary, not a native7328 effect claim.
    spell.effects = vec![SpellEffect {
        effect: 172,
        base_points: 35.0,
        trigger_spell: 15007,
        ..default()
    }];
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(vec![spell, aura]));
    let offered = offer(&mut h, target, conn);
    assert!(
        !h.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .has_spell(15007)
    );
    respond(
        &mut h,
        conn,
        ResurrectionResponse {
            caster: offered.caster,
            spell_id: 7328,
            accept: true,
        },
    );
    assert_eq!(
        h.app.world().get::<PlayerDeathState>(target).unwrap().state,
        DeathState::Alive
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .has_spell(15007)
    );
}

#[test]
fn rezrtap_damage_tap_projection_and_clear() {
    let mut h = harness(Vec::new(), &[], 2).expect("world fixture required");
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(42));
    let caster = h.caster;
    let npc = h.npc;
    crate::creature_tap::record_damage(h.app.world_mut(), npc, caster, 100.0, 10.0);
    let tap = h.app.world().get::<UnitTap>(npc).unwrap();
    assert!(!tap.denied(42, &[]));
    assert!(!tap.denied(99, &[42]));
    assert!(tap.denied(99, &[100]));
    crate::creature_tap::clear(h.app.world_mut(), npc);
    assert!(!h.app.world().get::<UnitTap>(npc).unwrap().denied(99, &[]));
}

/// A ghost at its graveyard's spirit healer, level `level`, with a connection
/// that can send `AcceptSpiritHealerResurrection`.
fn healer_fixture(level: u8) -> (Harness, Entity, Entity) {
    let sickness = world_db()
        .expect("world fixture")
        .store
        .get(15007)
        .expect("resurrection sickness fixture")
        .clone();
    let mut h = harness_with_plugins(vec![sickness], &[], 2, false, |app| {
        app.add_plugins(DeathPlugin);
    })
    .expect("world fixture required");
    let ghost = spawn_player(&mut h.app, 5, 3.0);
    h.app.world_mut().entity_mut(ghost).insert((
        shared::components::Level(level),
        PlayerDeathState {
            state: DeathState::Ghost,
            corpse: None,
            graveyard: Some(shared::death::Graveyard {
                id: 1,
                zone_id: 12,
                faction: 0,
                map_id: 0,
                x: 5.0,
                y: 0.0,
                z: 0.0,
            }),
        },
        MovementControl::default(),
        Health {
            current: 0.0,
            max: 1000.0,
        },
    ));
    let conn = spawn_peer_connection(&mut h.app, ghost, 99);
    h.app.world_mut().entity_mut(conn).insert((
        lightyear::prelude::MessageSender::<shared::protocol::DeathStateUpdate>::default(),
        lightyear::prelude::MessageReceiver::<shared::protocol::DeathStateUpdate>::default(),
        lightyear::prelude::MessageSender::<shared::protocol::AcceptSpiritHealerResurrection>::default(),
        lightyear::prelude::MessageReceiver::<shared::protocol::AcceptSpiritHealerResurrection>::default(),
    ));
    (h, ghost, conn)
}

fn accept_spirit_healer(h: &mut Harness, conn: Entity) {
    h.app
        .world_mut()
        .get_mut::<lightyear::prelude::MessageSender<shared::protocol::AcceptSpiritHealerResurrection>>(conn)
        .unwrap()
        .send::<shared::protocol::DeathChannel>(shared::protocol::AcceptSpiritHealerResurrection);
    h.advance(3);
}

fn has_sickness(h: &Harness, ghost: Entity) -> bool {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(ghost)
        .is_some_and(|auras| auras.has_spell(15007))
}

/// `WorldSession::HandleSpiritHealerActivate` → `Player::ResurrectPlayer(0.5f, true)`:
/// at or above the sickness level the player casts Resurrection Sickness (15007) on
/// itself.
#[test]
fn spirit_healer_resurrection_applies_resurrection_sickness() {
    let (mut h, ghost, conn) = healer_fixture(30);
    accept_spirit_healer(&mut h, conn);
    assert_eq!(
        h.app.world().get::<PlayerDeathState>(ghost).unwrap().state,
        DeathState::Alive
    );
    assert!(
        has_sickness(&h, ghost),
        "level 30 spirit healer resurrection"
    );
}

#[test]
fn spirit_healer_resurrection_below_sickness_level_has_no_sickness() {
    let (mut h, ghost, conn) = healer_fixture(5);
    accept_spirit_healer(&mut h, conn);
    assert_eq!(
        h.app.world().get::<PlayerDeathState>(ghost).unwrap().state,
        DeathState::Alive
    );
    assert!(
        !has_sickness(&h, ghost),
        "level 5 is below the sickness level"
    );
}
