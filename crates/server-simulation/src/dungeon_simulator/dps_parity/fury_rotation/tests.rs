use super::*;
use std::collections::BTreeMap;

#[test]
#[ignore = "requires the exact Fury72 player profile, traits and private world.db"]
fn fury_player_literal_fixture_casts_stance_recklessness_and_paid_rampage() {
    let result = super::super::measure(72, 1).expect("actual300s Fury player fixture");
    let actions = result["combat"]["actions"].as_array().unwrap();
    let casts = |spell| {
        actions
            .iter()
            .filter(|action| action["spell_id"] == spell)
            .map(|action| action["count"].as_u64().unwrap())
            .sum::<u64>()
    };
    assert_eq!(
        casts(386196),
        1,
        "literal precombat casts Berserker Stance once"
    );
    assert!(casts(1719) > 0, "literal Fury actually casts Recklessness");
    assert!(casts(184367) > 0, "actual native rage funds a paid Rampage");
    assert!(
        casts(190411) < 50,
        "literal Fury must not repeat251 Whirlwinds like native SBA"
    );
}

#[test]
fn fury_native_absent_movement_event_has_zero_distance() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    let actor = world.spawn_empty().id();
    let bindings = Bindings::default();
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(&world, actor, &bindings, encounter, 1);
    assert_eq!(facts.value("raid_event.movement.distance").unwrap(), 0.0);
    let dungeon = NativeFacts::new(
        &world,
        actor,
        &bindings,
        Encounter::Dungeon { started_at: 0.0 },
        1,
    );
    assert!(
        dungeon
            .value("raid_event.movement.distance")
            .unwrap_err()
            .contains("DUNGEON_TTD_UNAVAILABLE")
    );
}

struct State(BTreeMap<&'static str, f64>);
impl Facts for State {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.0
            .get(name)
            .copied()
            .ok_or_else(|| format!("missing test fact {name}"))
    }
}
impl ActionFacts for State {
    fn value_for(&self, _action: &Action, name: &str) -> Result<f64, String> {
        self.value(name)
    }
}

#[test]
fn fury_bladestorm_readiness_keeps_white_swings_and_selected_storm_bolt() {
    let facts = State(
        [
            ("buff.bladestorm.up", 1.0),
            ("talent.unrelenting_onslaught", 1.0),
        ]
        .into(),
    );
    let action = |name: &str| Action {
        source_row: None,
        name: name.into(),
        target: None,
        options: Default::default(),
    };
    assert!(!permits_bladestorm_action(&facts, &action("rampage")).unwrap());
    assert!(!permits_bladestorm_action(&facts, &action("bloodthirst")).unwrap());
    assert!(permits_bladestorm_action(&facts, &action("auto_attack")).unwrap());
    assert!(permits_bladestorm_action(&facts, &action("storm_bolt")).unwrap());
    let unselected = State(
        [
            ("buff.bladestorm.up", 1.0),
            ("talent.unrelenting_onslaught", 0.0),
        ]
        .into(),
    );
    assert!(!permits_bladestorm_action(&unselected, &action("storm_bolt")).unwrap());
    let inactive = State([("buff.bladestorm.up", 0.0)].into());
    assert!(permits_bladestorm_action(&inactive, &action("rampage")).unwrap());
}

#[test]
fn fury_literal_slayer_refreshes_enrage_before_spending_on_other_ready_actions() {
    let program = program().unwrap();
    let mut variables = program.state();
    let facts = State([("buff.enrage.remains", 0.0), ("gcd", 1.0)].into());
    let selected = program
        .select("slayer", &facts, &mut variables, &mut |action: &Action| {
            Ok(matches!(action.name.as_str(), "rampage" | "bloodthirst"))
        })
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "rampage");
}

#[test]
fn fury_literal_three_target_slayer_refreshes_whirlwind_before_recklessness() {
    let program = program().unwrap();
    let mut variables = program.state();
    let facts = State(
        [
            ("talent.improved_whirlwind", 1.0),
            ("buff.whirlwind.stack", 0.0),
        ]
        .into(),
    );
    let selected = program
        .select(
            "slayer_aoe",
            &facts,
            &mut variables,
            &mut |action: &Action| Ok(matches!(action.name.as_str(), "whirlwind" | "recklessness")),
        )
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "whirlwind");
}
