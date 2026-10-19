use super::*;
use crate::death::PlayerDeathState;
use shared::components::{CombatStatus, Mana};
use shared::death::DeathState;

struct CleanupFixture {
    entered: EnteredMatch,
    unentered: Traveler,
    other_players: Vec<Traveler>,
    other_copy: InMap,
}

#[test]
fn bg_end_cleanup_node_victory_resurrects_dead_participant_without_leaving() {
    let mut fixture = node_fixture();
    let (_, other_health) = drive_to_end(&mut fixture, 200);
    assert_untouched_actors_and_no_transfer(&mut fixture, other_health);
    assert_dead_participant_restored(&fixture);
}

#[test]
fn bg_end_cleanup_node_victory_stops_combat_without_healing_living_participant() {
    let mut fixture = node_fixture();
    let (living_health, other_health) = drive_to_end(&mut fixture, 200);
    assert_untouched_actors_and_no_transfer(&mut fixture, other_health);
    assert_living_participant_stopped_unhealed(&fixture, living_health);
}

#[test]
fn bg_end_cleanup_timeout_draw_restores_dead_and_stops_living_without_leaving() {
    let template = shared::battleground::warsong_gulch();
    let starts = shared::battleground::flag_rules(template.map_id)
        .unwrap()
        .starts;
    let mut fixture = prepare_fixture(&template, starts);
    let duration = (shared::battleground::FLAG_MATCH_MS / 1000 + 5) as usize;
    let (living_health, other_health) = drive_to_end(&mut fixture, duration);
    assert_eq!(
        battleground(&fixture.entered.h, &fixture.entered.players[0]).outcome,
        Some(BgOutcome::Draw)
    );
    assert_untouched_actors_and_no_transfer(&mut fixture, other_health);
    assert_dead_participant_restored(&fixture);
    assert_living_participant_stopped_unhealed(&fixture, living_health);
}

fn node_fixture() -> CleanupFixture {
    let template = shared::battleground::arathi_basin();
    let starts = node_rules(template.map_id).unwrap().starts;
    let mut fixture = prepare_fixture(&template, starts);
    let actor = &fixture.entered.players[2];
    for point in source_battleground_objects(template.map_id).0 {
        let (object, at) = objects_in(
            &mut fixture.entered.h,
            fixture.entered.copy,
            point.info.entry,
        )[0];
        fixture.entered.h.walk_to(actor, at);
        use_object(&mut fixture.entered.h, actor, object);
    }
    fixture
}

fn prepare_fixture(template: &BgTemplate, starts: [u32; 2]) -> CleanupFixture {
    let mut entered = enter_source_match_with(template, starts, |app| {
        app.add_plugins(crate::death::DeathPlugin);
    });
    assert_eq!(
        battleground(&entered.h, &entered.players[0]).phase,
        BgPhase::InProgress
    );
    let level = entered
        .h
        .app
        .world()
        .get::<shared::components::Level>(entered.players[0].player)
        .unwrap()
        .0;
    let unentered = entered
        .h
        .spawn_at_level(1000, level, STORMWIND, STOCKADE_ENTRANCE_TRIGGER);
    seed_mana_and_life(&mut entered.h, &unentered);
    let killer_index = entered
        .teams
        .iter()
        .position(|team| *team != entered.teams[0])
        .unwrap();
    let killer = entered.players[killer_index].player;
    deal_lethal_damage(&mut entered.h, killer, &unentered);
    seed_mana_and_life(&mut entered.h, &entered.players[0]);
    deal_lethal_damage(&mut entered.h, killer, &entered.players[0]);
    seed_mana_and_life(&mut entered.h, &entered.players[1]);
    entered
        .h
        .app
        .world_mut()
        .get_mut::<Health>(entered.players[1].player)
        .unwrap()
        .current = 40.0;
    let store = entered.h.app.world().resource::<MapStore>().clone();
    let other_copy = entered
        .h
        .app
        .world_mut()
        .resource_mut::<InstanceManager>()
        .create_battleground_map(&store, template.map_id, &[1001, 1002], 2, at(0.0))
        .unwrap();
    let other_players: Vec<_> = [1001, 1002]
        .into_iter()
        .map(|id| {
            entered
                .h
                .spawn_at_level(id, level, other_copy, entered.origin)
        })
        .collect();
    for who in &other_players {
        seed_mana_and_life(&mut entered.h, who);
        entered
            .h
            .app
            .world_mut()
            .resource_mut::<InstanceManager>()
            .add_player(other_copy, who.player);
    }
    use_second_frames(&mut entered.h);
    CleanupFixture {
        entered,
        unentered,
        other_players,
        other_copy,
    }
}

fn seed_mana_and_life(h: &mut Harness, who: &Traveler) {
    h.app.world_mut().entity_mut(who.player).insert((
        PlayerDeathState::default(),
        Mana {
            current: 30.0,
            max: 200.0,
        },
    ));
}

fn deal_lethal_damage(h: &mut Harness, attacker: Entity, who: &Traveler) {
    let damage = h.app.world().get::<Health>(who.player).unwrap().max * 2.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), attacker, who.player, damage);
    h.advance(2);
    let death = h.app.world().get::<PlayerDeathState>(who.player).unwrap();
    assert_eq!(death.state, DeathState::Dead);
    assert!(death.corpse.is_some());
    assert_eq!(
        h.app.world().get::<Health>(who.player).unwrap().current,
        0.0
    );
}

fn drive_to_end(fixture: &mut CleanupFixture, frame_budget: usize) -> (f32, f32) {
    let h = &mut fixture.entered.h;
    let living = fixture.entered.players[1].player;
    let attacker_index = fixture
        .entered
        .teams
        .iter()
        .enumerate()
        .find(|(index, team)| *index != 0 && **team != fixture.entered.teams[1])
        .map(|(index, _)| index)
        .expect("living assigned opponent");
    let attacker = fixture.entered.players[attacker_index].player;
    let other = fixture.other_players[0].player;
    let other_attacker = fixture.other_players[1].player;
    for _ in 0..frame_budget {
        assert_eq!(
            battleground(h, &fixture.entered.players[0]).phase,
            BgPhase::InProgress
        );
        // Real hits keep both combat references fresh through the terminal tick.
        crate::combat::deal_direct_damage(h.app.world_mut(), attacker, living, 0.01);
        crate::combat::deal_direct_damage(h.app.world_mut(), other_attacker, other, 0.01);
        assert_eq!(
            h.app.world().get::<CombatStatus>(living),
            Some(&CombatStatus(true))
        );
        assert_eq!(
            h.app.world().get::<CombatStatus>(other),
            Some(&CombatStatus(true))
        );
        let living_health = h.app.world().get::<Health>(living).unwrap().current;
        let other_health = h.app.world().get::<Health>(other).unwrap().current;
        h.receive_pvp_updates(1);
        if battleground(h, &fixture.entered.players[0]).phase == BgPhase::Ended {
            return (living_health, other_health);
        }
    }
    panic!("ordinary source lifecycle did not finish within {frame_budget} seconds");
}

fn assert_untouched_actors_and_no_transfer(fixture: &mut CleanupFixture, other_health: f32) {
    let h = &mut fixture.entered.h;
    for who in &fixture.entered.players {
        assert_eq!(h.map_of(who), Some(fixture.entered.copy));
        assert!(
            h.new_worlds(who).is_empty(),
            "completion never leaves the copy"
        );
        assert_eq!(battleground(h, who).phase, BgPhase::Ended);
    }
    let unentered = fixture.unentered.player;
    assert_eq!(h.map_of(&fixture.unentered), Some(STORMWIND));
    assert_eq!(h.app.world().get::<Health>(unentered).unwrap().current, 0.0);
    let death = h.app.world().get::<PlayerDeathState>(unentered).unwrap();
    assert_eq!(death.state, DeathState::Dead);
    assert!(death.corpse.is_some());
    let other = fixture.other_players[0].player;
    assert_eq!(
        h.map_of(&fixture.other_players[0]),
        Some(fixture.other_copy)
    );
    assert_eq!(
        h.app.world().get::<Health>(other).unwrap().current,
        other_health
    );
    assert_eq!(h.app.world().get::<Mana>(other).unwrap().current, 30.0);
    assert_eq!(
        h.app.world().get::<CombatStatus>(other),
        Some(&CombatStatus(true))
    );
}

fn assert_dead_participant_restored(fixture: &CleanupFixture) {
    let world = fixture.entered.h.app.world();
    let dead = fixture.entered.players[0].player;
    let health = world.get::<Health>(dead).unwrap();
    assert_eq!(
        health.current, health.max,
        "Ended snapshot must already expose full resurrection"
    );
    let mana = world.get::<Mana>(dead).unwrap();
    assert_eq!(mana.current, mana.max);
    let death = world.get::<PlayerDeathState>(dead).unwrap();
    assert_eq!(death.state, DeathState::Alive);
    assert!(death.corpse.is_none());
    assert!(death.graveyard.is_none());
}

fn assert_living_participant_stopped_unhealed(fixture: &CleanupFixture, expected_health: f32) {
    let world = fixture.entered.h.app.world();
    let living = fixture.entered.players[1].player;
    assert_eq!(
        world.get::<Health>(living).unwrap().current,
        expected_health
    );
    assert_eq!(world.get::<Mana>(living).unwrap().current, 30.0);
    assert_eq!(
        world.get::<CombatStatus>(living),
        Some(&CombatStatus(false)),
        "Ended snapshot must already expose CombatStop"
    );
}
