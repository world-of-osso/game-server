//! Native ONLY_ON_PLAYER is unit identity, not friendliness or player control.
use super::*;
use shared::components::{Health, ModelDisplay, UnitFactionTemplate};

#[derive(Clone, Copy)]
struct PlayerToy {
    item: u32,
    spell: u32,
    recovery_ms: u32,
}

const HEARTBREAKER: PlayerToy = PlayerToy {
    item: 50471,
    spell: 71909,
    recovery_ms: 60_000,
};
const BBF: PlayerToy = PlayerToy {
    item: 205045,
    spell: 408067,
    recovery_ms: 86_400_000,
};

fn select_actor(h: &mut Harness, actor: Entity) {
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(actor);
}

fn prove_player_toy_refuses_npc(toy: PlayerToy) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("player-target.redb"));
    learn_effect_toy(&mut h, character, toy.item);
    // Harness NPC6 uses its authored model10913; it remains an NPC, even friendly.
    let faction = *h.app.world().get::<UnitFactionTemplate>(h.caster).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((faction, ModelDisplay(10913)));
    assert!(
        h.app
            .world()
            .get::<shared::components::Player>(h.npc)
            .is_none()
    );
    let health = *h.app.world().get::<Health>(h.npc).unwrap();
    let npc = h.npc;
    select_actor(&mut h, npc);
    let results = activate(&mut h, toy.item);
    assert!(
        results
            .iter()
            .any(|result| result.operation == ToyOperation::Use && result.error.is_some()),
        "item{} accepted NPC: {results:?}; cooldowns{:?}",
        toy.item,
        h.inbox.cooldowns
    );
    h.advance(10);
    assert!(!crate::auras::has_aura(h.app.world(), npc, toy.spell));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == toy.spell));
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|clock| clock.spell_id == toy.spell && clock.duration_ms > 0)
    );
    assert_eq!(*h.app.world().get::<Health>(npc).unwrap(), health);
    assert_eq!(
        h.app.world().get::<ModelDisplay>(npc),
        Some(&ModelDisplay(10913))
    );
    assert!(
        toy_store::load(&h.app.world().resource::<GameDatabase>().db, 1).contains_key(&toy.item)
    );

    assert_player_use_after_refusal(&mut h, npc, toy);
}

fn assert_player_use_after_refusal(h: &mut Harness, npc: Entity, toy: PlayerToy) {
    // The refused attempt must not consume the source item recovery.
    let friend = spawn_player(&mut h.app, 8, 6.0);
    h.app
        .world_mut()
        .entity_mut(friend)
        .insert(CombatStatus(false));
    select_actor(h, friend);
    assert_eq!(activate(h, toy.item)[0].error, None);
    h.advance(10);
    assert!(crate::auras::has_aura(h.app.world(), friend, toy.spell));
    assert!(!crate::auras::has_aura(h.app.world(), npc, toy.spell));
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == toy.spell && go.hit_targets.contains(&friend.to_bits()))
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|clock| clock.spell_id == toy.spell
                && !clock.is_gcd
                && clock.duration_ms == toy.recovery_ms)
    );
}

#[test]
#[ignore = "player-only source qualification; run explicitly"]
fn toy_coverage_player_only_heartbreaker_refuses_npc_then_uses_player() {
    prove_player_toy_refuses_npc(HEARTBREAKER);
}

#[test]
#[ignore = "player-only source qualification; run explicitly"]
fn toy_coverage_player_only_bbf_refuses_npc_then_uses_player() {
    prove_player_toy_refuses_npc(BBF);
}

#[test]
#[ignore = "player-only source qualification; run explicitly"]
fn toy_coverage_player_only_native_area_filters_before_authored_cap() {
    let mut h = harness(Vec::new(), &[7645], 8).unwrap();
    let player = spawn_player(&mut h.app, 8, 6.0);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(UnitFactionTemplate(
            crate::faction::faction_template_for_race(2),
        ));
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(ModelDisplay(10913));
    let spell = world_db().unwrap().store.get(7645).unwrap().clone();
    assert_eq!(spell.misc.as_ref().unwrap().attributes[3] & 0x100, 0x100);
    let cap = spell.target_restrictions.as_ref().unwrap().max_targets;
    assert_eq!(cap, 1);
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.implicit_targets == [22, 15])
        .unwrap();
    let npc = h.npc;
    let caster = h.caster;
    assert!(crate::spell_cast::targets::is_hostile(
        h.app.world(),
        caster,
        npc
    ));
    assert!(crate::spell_cast::targets::is_hostile(
        h.app.world(),
        caster,
        player
    ));
    let recipients = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        caster,
        None,
        &spell,
        effect,
        cap,
        false,
    )
    .unwrap();
    assert_eq!(
        recipients,
        vec![player],
        "closer NPC must not take the native one-unit slot"
    );
}

#[test]
#[ignore = "player-only source qualification; run explicitly"]
fn toy_coverage_player_only_native_self_keeps_player_not_npc() {
    let mut h = harness(Vec::new(), &[147643], 8).unwrap();
    let spell = world_db().unwrap().store.get(147643).unwrap().clone();
    assert_eq!(spell.misc.as_ref().unwrap().attributes[3] & 0x100, 0x100);
    let effect = &spell.effects[0];
    assert_eq!(effect.implicit_targets, [1, 0]);
    let caster = h.caster;
    let npc = h.npc;
    let player_recipients = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        caster,
        None,
        &spell,
        effect,
        0,
        false,
    )
    .unwrap();
    assert_eq!(player_recipients, vec![caster]);
    let npc_recipients = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        npc,
        None,
        &spell,
        effect,
        0,
        false,
    )
    .unwrap();
    assert!(
        npc_recipients.is_empty(),
        "NPC self incorrectly satisfies ONLY_ON_PLAYER: {npc_recipients:?}"
    );
}

#[test]
#[ignore = "player-only source qualification; run explicitly"]
fn toy_coverage_player_only_native_destination_is_not_a_unit_recipient() {
    let mut h = harness(Vec::new(), &[91756], 8).unwrap();
    let spell = world_db().unwrap().store.get(91756).unwrap().clone();
    assert_eq!(spell.misc.as_ref().unwrap().attributes[3] & 0x100, 0x100);
    let effect = &spell.effects[0];
    assert_eq!(effect.implicit_targets, [18, 0]);
    let caster = h.caster;
    let recipients = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        caster,
        None,
        &spell,
        effect,
        0,
        false,
    )
    .unwrap();
    assert!(recipients.is_empty());
}
