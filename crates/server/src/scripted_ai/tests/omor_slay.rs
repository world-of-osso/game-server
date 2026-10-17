//! TC a352b1fa Omor KilledUnit: actual lethal damage, not last-damager inference.
use super::*;

fn slay_fixture() -> (Harness, Entity) {
    let (mut h, boss, _) = super::omor::orbital_fixture();
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        Position {
            x: origin.x + 5.0,
            ..origin
        },
        crate::persistence::ClientId(1001),
    ));
    h.app.init_resource::<crate::death::DeathServerState>();
    h.app
        .init_resource::<crate::death::DeathTransitionCounters>();
    h.app.add_systems(
        FixedUpdate,
        crate::death::process_player_death_transitions.after(crate::combat::auto_attack_system),
    );
    (h, boss)
}

fn slay_count(h: &Harness, boss: Entity) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&17308][&3];
    let name = &h
        .app
        .world()
        .get::<shared::components::Npc>(boss)
        .unwrap()
        .name;
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            let authored = texts
                .iter()
                .any(|text| line.content == text.male_text || line.content == text.female_text);
            let from_boss = line.channel == shared::protocol::ChatType::MonsterYell(boss.to_bits())
                && &line.sender == name;
            authored && from_boss
        })
        .count()
}

fn cast_ordinary_shadow_bolt(h: &mut Harness, boss: Entity, target: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30686, Some(target), false)
        .expect("actual native Omor Shadow Bolt admission");
    h.advance(4.2); // Native cast/flight/recovery; do not force a hit or reset cooldowns.
}

#[test]
#[ignore = "source ordinary30686 lethal player KilledUnit acceptance pending implementation"]
fn encounter_coverage_omor_slay_ordinary_shadow_bolt_speaks_once_on_player_death() {
    let (mut h, boss) = slay_fixture();
    let player = h.player();
    let before = h.health(player).current;
    cast_ordinary_shadow_bolt(&mut h, boss, player);
    assert!(h.health(player).current > 0.0, "initial hit is nonlethal");
    assert!(
        h.health(player).current < before,
        "actual source30686 damage before slay test"
    );
    assert_eq!(
        slay_count(&h, boss),
        0,
        "nonlethal damage cannot speak slay"
    );
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 1.0;
    // Native miss/resist rolls remain; bounded retries use ordinary casts and clocks.
    for _ in 0..8 {
        if h.health(player).current <= 0.0 {
            break;
        }
        cast_ordinary_shadow_bolt(&mut h, boss, player);
    }
    assert_eq!(h.health(player).current, 0.0, "actual lethal30686 impact");
    let death = h
        .app
        .world()
        .get::<crate::death::PlayerDeathState>(player)
        .expect("production death transition creates corpse state");
    assert_eq!(death.state, shared::death::DeathState::Dead);
    assert!(
        death.corpse.is_some(),
        "source death boundary precedes slay hook"
    );
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(30686)
                && event.amount > 0
                && event.kind == shared::protocol::CombatLogKind::Damage
        }),
        "owned native Shadow Bolt damage receipt"
    );
    assert_eq!(
        slay_count(&h, boss),
        1,
        "source KilledUnit emits exactly one authored Talk3"
    );
    h.app
        .world_mut()
        .write_message(crate::death::PlayerDeathTransition { player });
    h.advance(1.0);
    assert_eq!(
        slay_count(&h, boss),
        1,
        "repeated death/idle ticks cannot replay slay"
    );
    assert!(h.inbox(1).chat.is_empty(), "other copy cannot hear slay");
}
