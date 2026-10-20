use super::*;
use crate::spell_cast::powers::{PowerPool, PowerPools};
use crate::stats::CharacterStats;
use shared::components::{CombatStatus, Health, Mana, PowerType};

const PREPARATION: u32 = 44521;
const FIREBALL: u32 = 133;
const SINISTER_STRIKE: u32 = 1752;
const BASELINE_COSTS: [f32; 2] = [20.0, 45.0];

#[test]
fn bg_preparation_aura_native44521_qualification_changes_cost_and_affordability_then_restores() {
    let (mut h, players, _database) = fixture(1018);
    let who = &players[0];
    assert_eq!(costs(&h, who), BASELINE_COSTS);
    h.app
        .world_mut()
        .get_mut::<Mana>(who.player)
        .unwrap()
        .current = 0.0;
    crate::spell_cast::powers::change_power(
        h.app.world_mut(),
        who.player,
        PowerType::Energy,
        -100.0,
    );
    assert!(!affordable(&h, who));
    apply_preparation(&mut h, who);
    assert_eq!(costs(&h, who), [0.0; 2]);
    assert!(affordable(&h, who));
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(who.player)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == PREPARATION && aura.expires_at.is_none())
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        who.player,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == PREPARATION,
    );
    assert_eq!(costs(&h, who), BASELINE_COSTS);
    assert!(!affordable(&h, who));
}

#[test]
fn bg_preparation_aura_arena_ack_retains32727_without44521() {
    let (mut h, players, _database) = pending_arena(25);
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let spells = [
        32727,
        PREPARATION,
        DESERTER_SPELL_ID,
        DUNGEON_DESERTER_SPELL_ID,
    ]
    .into_iter()
    .map(|id| data.store.get(id).unwrap().clone())
    .collect();
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(spells));
    for who in &players {
        let match_id = battleground(&h, who).match_id;
        let updates = request_port(&mut h, who, match_id);
        assert!(updates.iter().all(|update| update.error.is_none()));
        assert_eq!(h.new_worlds(who).len(), 1);
        h.ack(who);
        assert!(aura_id(&h, who, PREPARATION).is_none());
        assert!(
            aura_id(&h, who, 32727).is_some(),
            "existing arena preparation remains"
        );
    }
}

#[test]
fn bg_preparation_aura_declined_pending_invitation_cannot_grant_zero_cost() {
    for queue in [1018, 32] {
        let (mut h, players, _database) = fixture(queue);
        let who = &players[0];
        let match_id = battleground(&h, who).match_id;
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id,
                accepted: false,
            });
        let updates = h.receive_pvp_updates(5);
        assert!(
            updates[&who.connection]
                .iter()
                .all(|update| update.error.is_none())
        );
        assert!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
        assert!(aura_id(&h, who, PREPARATION).is_none());
        assert_eq!(costs(&h, who), BASELINE_COSTS);
        assert!(h.new_worlds(who).is_empty());
    }
}

#[test]
#[ignore = "RED: ordinary Waiting ACK does not grant native Preparation44521"]
fn bg_preparation_aura_concrete_actual_ack_grants_once_then_start_restores_cost() {
    assert_arrival(1018);
}
#[test]
#[ignore = "RED: ordinary Waiting ACK does not grant native Preparation44521"]
fn bg_preparation_aura_random_actual_ack_grants_once_then_start_restores_cost() {
    assert_arrival(32);
}
#[test]
#[ignore = "RED: natural ordinary BG start does not remove44521"]
fn bg_preparation_aura_concrete_start_clears_only_current_arrived_copy() {
    assert_start(1018);
}
#[test]
#[ignore = "RED: natural ordinary BG start does not remove44521"]
fn bg_preparation_aura_random_start_clears_only_current_arrived_copy() {
    assert_start(32);
}
#[test]
#[ignore = "RED: ordinary successful BG exit does not clear44521 role aura"]
fn bg_preparation_aura_concrete_successful_exit_restores_cost() {
    assert_exit(1018, false);
}
#[test]
#[ignore = "RED: ordinary successful BG exit does not clear44521 role aura"]
fn bg_preparation_aura_random_successful_exit_restores_cost() {
    assert_exit(32, false);
}
#[test]
fn bg_preparation_aura_concrete_refused_exit_retains_cost_and_aura() {
    assert_exit(1018, true);
}
#[test]
fn bg_preparation_aura_random_refused_exit_retains_cost_and_aura() {
    assert_exit(32, true);
}
#[test]
fn bg_preparation_aura_concrete_late_in_progress_arrival_never_gets_zero_cost() {
    assert_late_arrival(1018);
}
#[test]
fn bg_preparation_aura_random_late_in_progress_arrival_never_gets_zero_cost() {
    assert_late_arrival(32);
}

fn fixture(queue: u32) -> (Harness, Vec<Traveler>, tempfile::NamedTempFile) {
    let (mut h, players, database) = pending_battleground(queue);
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let spells = [
        PREPARATION,
        FIREBALL,
        SINISTER_STRIKE,
        DESERTER_SPELL_ID,
        DUNGEON_DESERTER_SPELL_ID,
    ]
    .into_iter()
    .map(|id| data.store.get(id).unwrap().clone())
    .collect();
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(spells));
    for who in &players {
        h.app.world_mut().entity_mut(who.player).insert((
            CharacterStats {
                base_mana: 2000.0,
                ..Default::default()
            },
            Mana {
                current: 35.0,
                max: 100.0,
            },
            PowerPools {
                order: vec![PowerType::Mana, PowerType::Energy],
                pools: vec![PowerPool {
                    power: PowerType::Energy,
                    current: 100.0,
                    max: 100.0,
                    regen_per_sec: 0.0,
                }],
                ..Default::default()
            },
        ));
    }
    (h, players, database)
}

fn costs(h: &Harness, who: &Traveler) -> [f32; 2] {
    [FIREBALL, SINISTER_STRIKE].map(|id| {
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .unwrap();
        let costs = crate::spell_cast::applicable_costs(h.app.world(), who.player, spell).unwrap();
        assert_eq!(costs.len(), 1);
        costs[0].1
    })
}

fn affordable(h: &Harness, who: &Traveler) -> bool {
    [FIREBALL, SINISTER_STRIKE].into_iter().all(|id| {
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .unwrap();
        crate::spell_cast::spell_costs(h.app.world(), who.player, spell).is_ok()
    })
}

fn aura_id(h: &Harness, who: &Traveler, spell: u32) -> Option<u32> {
    h.app
        .world()
        .get::<ActiveAuras>(who.player)?
        .iter()
        .find(|aura| aura.spell_id == spell)
        .map(|aura| aura.instance_id)
}

fn apply_preparation(h: &mut Harness, who: &Traveler) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(PREPARATION)
        .unwrap();
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: who.player,
            target: who.player,
            spell_id: PREPARATION,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(aura_id(h, who, PREPARATION).is_some());
}

fn enter(h: &mut Harness, who: &Traveler) {
    let invitation = battleground(h, who).clone();
    let updates = request_port(h, who, invitation.match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "entry: {updates:?}"
    );
    assert_eq!(h.new_worlds(who).len(), 1);
    assert!(
        aura_id(h, who, PREPARATION).is_none(),
        "pending transfer is not arrival"
    );
    assert_eq!(costs(h, who), BASELINE_COSTS);
    h.ack(who);
}

fn assert_arrival(queue: u32) {
    let (mut h, players, _database) = fixture(queue);
    let who = &players[0];
    assert!(aura_id(&h, who, PREPARATION).is_none());
    apply_sourced_aura(&mut h, who, DUNGEON_DESERTER_SPELL_ID);
    let unrelated = aura_id(&h, who, DUNGEON_DESERTER_SPELL_ID);
    for entrant in &players {
        enter(&mut h, entrant);
    }
    assert_eq!(
        battleground(&h, who).phase,
        shared::battleground::BgPhase::Waiting
    );
    let preparation = aura_id(&h, who, PREPARATION).expect("actual Waiting ACK grants44521");
    assert_eq!(costs(&h, who), [0.0; 2]);
    h.advance(5);
    assert_eq!(
        aura_id(&h, who, PREPARATION),
        Some(preparation),
        "not reapplied every frame"
    );
    super::super::super::advance_source_preparation(&mut h);
    assert_eq!(
        battleground(&h, who).phase,
        shared::battleground::BgPhase::InProgress
    );
    assert!(aura_id(&h, who, PREPARATION).is_none());
    assert_eq!(costs(&h, who), BASELINE_COSTS);
    assert_eq!(aura_id(&h, who, DUNGEON_DESERTER_SPELL_ID), unrelated);
    assert_eq!(
        h.app.world().get::<Mana>(who.player).unwrap().current,
        35.0,
        "no arena reset path"
    );
}

fn assert_start(queue: u32) {
    let (mut h, players, _database) = fixture(queue);
    for who in &players[..19] {
        enter(&mut h, who);
        apply_preparation(&mut h, who);
    }
    let who = &players[0];
    let foreign = &players[18];
    let copy = h.map_of(who).unwrap();
    h.app.world_mut().entity_mut(foreign.player).insert(InMap {
        instance_id: copy.instance_id + 1000,
        ..copy
    });
    let foreign_aura = aura_id(&h, foreign, PREPARATION);
    super::super::super::advance_source_preparation(&mut h);
    assert_eq!(
        battleground(&h, who).phase,
        shared::battleground::BgPhase::InProgress
    );
    assert!(
        aura_id(&h, who, PREPARATION).is_none(),
        "natural start clears44521"
    );
    assert_eq!(costs(&h, who), BASELINE_COSTS);
    assert_eq!(
        aura_id(&h, foreign, PREPARATION),
        foreign_aura,
        "other copy unchanged"
    );
    assert!(
        aura_id(&h, &players[19], PREPARATION).is_none(),
        "unentered actor unchanged"
    );
}

fn assert_exit(queue: u32, refused: bool) {
    let (mut h, players, _database) = fixture(queue);
    let who = &players[0];
    enter(&mut h, who);
    apply_preparation(&mut h, who);
    apply_sourced_aura(&mut h, who, DUNGEON_DESERTER_SPELL_ID);
    let unrelated = aura_id(&h, who, DUNGEON_DESERTER_SPELL_ID);
    let preparation = aura_id(&h, who, PREPARATION);
    let copy = h.map_of(who);
    h.app
        .world_mut()
        .entity_mut(who.player)
        .insert(CombatStatus(refused));
    h.app
        .world_mut()
        .entity_mut(who.connection)
        .insert(pair::<LeaveBattleground>());
    let match_id = battleground(&h, who).match_id;
    h.app
        .world_mut()
        .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(LeaveBattleground { match_id });
    let updates = h.receive_pvp_updates(5);
    if refused {
        assert!(
            updates[&who.connection]
                .iter()
                .any(|update| update.error.is_some())
        );
        assert_eq!(h.map_of(who), copy);
        assert_eq!(aura_id(&h, who, PREPARATION), preparation);
        assert_eq!(costs(&h, who), [0.0; 2]);
        assert!(h.new_worlds(who).is_empty());
    } else {
        assert!(
            updates[&who.connection]
                .iter()
                .all(|update| update.error.is_none())
        );
        assert_eq!(h.new_worlds(who).len(), 1);
        h.ack(who);
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(
            aura_id(&h, who, PREPARATION).is_none(),
            "successful BG exit clears preparation role"
        );
        assert_eq!(costs(&h, who), BASELINE_COSTS);
    }
    assert_eq!(aura_id(&h, who, DUNGEON_DESERTER_SPELL_ID), unrelated);
    assert!(h.app.world().get::<Health>(who.player).unwrap().current > 0.0);
}

fn assert_late_arrival(queue: u32) {
    let (mut h, players, _database) = fixture(queue);
    for who in &players[1..] {
        enter(&mut h, who);
    }
    super::super::super::advance_source_preparation(&mut h);
    assert_eq!(
        battleground(&h, &players[1]).phase,
        shared::battleground::BgPhase::InProgress
    );
    enter(&mut h, &players[0]);
    assert!(aura_id(&h, &players[0], PREPARATION).is_none());
    assert_eq!(costs(&h, &players[0]), BASELINE_COSTS);
}
