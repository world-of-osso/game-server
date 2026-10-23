//! Action duration, scheduled tick and actual native admission boundaries.
use super::*;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

fn evoker() -> (Harness, Entity, Bindings) {
    let (mut h, actor) = crate::followers::kit_tests::at_level(215240, 90, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.app
        .world_mut()
        .entity_mut(actor)
        .insert(crate::networking::CombatTarget(Some(h.npc)));
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::Health {
            current: 100_000_000.0,
            max: 100_000_000.0,
        });
    let b = Bindings {
        actions: [
            ("living_flame".into(), 361469),
            ("disintegrate".into(), 356995),
        ]
        .into(),
        global_cooldown_spell: 361469,
        ..Default::default()
    };
    (h, actor, b)
}

#[test]
#[ignore = "requires native DB inputs: execute_time observes real cast and channel duration"]
fn native_apl_extensions_execute_time_matches_actual_flame_and_disintegrate() {
    for (name, spell) in [("living_flame", 361469), ("disintegrate", 356995)] {
        let (mut h, actor, b) = evoker();
        let clock = fixture(h.app.world());
        let proposed = action(name);
        let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
        let expected = f.value_for(&proposed, "execute_time").unwrap();
        let gcd = f.value_for(&proposed, "gcd").unwrap();
        crate::spell_cast::creature_cast(h.app.world_mut(), actor, spell, Some(h.npc), false)
            .unwrap();
        let cast = h
            .app
            .world()
            .get::<CastState>(actor)
            .expect("real native action starts a cast or channel");
        let duration = f64::from(cast.duration);
        let actual = if name == "disintegrate" {
            duration
        } else {
            duration.max(gcd)
        };
        assert!(
            (expected - actual).abs() < 0.001,
            "{name}: fact{expected}, actual cast/channel{duration}, gcd{gcd}"
        );
    }
}

fn disintegrate_hits(h: &Harness, actor: Entity) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(actor.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.spell_id == Some(356995)
        })
        .count()
}

#[test]
#[ignore = "requires native DB inputs: ticks_remain reads unpaid native schedule"]
fn native_apl_extensions_ticks_remain_matches_future_disintegrate_hits() {
    let (mut h, actor, b) = evoker();
    let clock = fixture(h.app.world());
    let proposed = action("disintegrate");
    crate::spell_cast::creature_cast(h.app.world_mut(), actor, 356995, Some(h.npc), false).unwrap();
    h.advance(1);
    assert!(
        disintegrate_hits(&h, actor) > 0,
        "real native tick-zero landed"
    );
    let paid = disintegrate_hits(&h, actor);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    let remaining = f.value_for(&proposed, "ticks_remain").unwrap();
    assert!(remaining > 0.0, "real channel has future ticks");
    h.advance(100);
    assert_eq!(remaining, (disintegrate_hits(&h, actor) - paid) as f64);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value_for(&proposed, "ticks_remain").unwrap(), 0.0);
}

#[test]
#[ignore = "requires native DB inputs: ready includes target range, usable_in is cooldown time"]
fn native_apl_extensions_action_ready_distinguishes_cooldown_from_actual_range() {
    let (mut h, actor) = monk();
    let target = h.npc;
    h.app
        .world_mut()
        .entity_mut(actor)
        .insert(crate::networking::CombatTarget(Some(target)));
    let b = bindings();
    let clock = fixture(h.app.world());
    cast(&mut h, actor, PROVOKE, target);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value("action.provoke.ready").unwrap(), 0.0);
    let remains = f.value("action.provoke.usable_in").unwrap();
    assert!(remains > 0.0);
    assert_eq!(remains, f.value("cooldown.provoke.remains").unwrap());
    h.advance((remains * 20.0).ceil() as usize + 2);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value("action.provoke.usable_in").unwrap(), 0.0);
    assert_eq!(f.value("action.provoke.ready").unwrap(), 1.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap()
        .x += 1000.0;
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value("cooldown.provoke.up").unwrap(), 1.0);
    assert_eq!(f.value("action.provoke.ready").unwrap(), 0.0);
    assert_eq!(f.value("action.provoke.usable_in").unwrap(), 0.0);
    assert!(matches!(
        crate::spell_cast::creature_cast(h.app.world_mut(), actor, PROVOKE, Some(target), false),
        Err(crate::spell_cast::CastFailure::OutOfRange)
    ));
}
