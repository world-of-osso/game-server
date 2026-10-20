//! TC a352b1fa Blood Furnace The Maker17381 (boss_the_maker.cpp).
use super::*;

const MAKER: u32 = 17381;
const BLOOD_FURNACE: u32 = 542;
const NORMAL_BEAKER: u32 = 30925;
const HEROIC_BEAKER: u32 = 40059;
const DOMINATION: u32 = 30923;

fn maker_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    bound_boss_at_difficulty(MAKER, BLOOD_FURNACE, 0, difficulty, None)
}

/// Advance `secs`, reporting whether `target` was ever in a knockback flight.
fn observe_knockback(h: &mut Harness, target: Entity, secs: f32) -> bool {
    let mut knocked = false;
    for _ in 0..(secs / 0.05).round() as usize {
        h.advance(0.05);
        knocked |= h
            .app
            .world()
            .get::<crate::knockback::KnockbackFlight>(target)
            .is_some();
    }
    knocked
}

#[test]
fn encounter_coverage_maker_native_inputs_qualify() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = maker_copy(difficulty);
        let world = h.app.world();
        assert_eq!(
            world
                .resource::<crate::maps::MapStore>()
                .instance_scripts
                .get(&BLOOD_FURNACE)
                .map(String::as_str),
            Some("instance_blood_furnace")
        );
        assert_eq!(
            world
                .get::<shared::components::Npc>(boss)
                .unwrap()
                .template_id,
            MAKER,
            "TDB spawnDifficulties1,2 spawn in difficulty{difficulty}"
        );
        let texts = &world.resource::<CreatureTexts>().groups[&MAKER];
        let counts: Vec<_> = (0..=2).map(|group| texts[&group].len()).collect();
        assert_eq!(counts, [3, 2, 1], "native aggro/slay/death groups");
        assert!(
            texts
                .values()
                .flatten()
                .all(|text| text.chat_type == 14 && text.text_range == 0),
            "native MonsterYell/TextRange0"
        );
        let spells = world.resource::<SpellStore>();
        assert!(
            spells.get(NORMAL_BEAKER).is_none(),
            "retail70448 has no Normal beaker30925"
        );
        assert!(spells.get(HEROIC_BEAKER).is_some());
        assert!(spells.get(DOMINATION).is_some());
        // HARNESS SETUP: production state transition proves ordinal0 resolves encounter1922.
        assert!(instance_script::set_boss_state(
            h.app.world_mut(),
            map,
            0,
            EncounterState::InProgress
        ));
        h.advance(0.05);
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 1922 && start.difficulty_id == difficulty),
            "ordinal0 -> DungeonEncounter1922 in difficulty{difficulty}"
        );
        assert!(h.inbox(1).starts.is_empty(), "other copy isolated");
    }
}

#[test]
fn encounter_coverage_maker_native_heroic_beaker_and_normal_absence_qualify() {
    let (mut h, boss, _) = maker_copy(2);
    let player = h.player();
    let before = *h.app.world().get::<Position>(player).unwrap();
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, HEROIC_BEAKER, Some(player), false)
        .expect("actual ordinary Heroic40059 admission");
    let knocked = observe_knockback(&mut h, player, 4.0);
    eprintln!(
        "MAKER beaker gos={:?} log={:?}",
        spell_gos(&h, boss, HEROIC_BEAKER),
        h.inbox(0)
            .log
            .iter()
            .filter(|event| event.spell_id == Some(HEROIC_BEAKER))
            .collect::<Vec<_>>()
    );
    assert_eq!(spell_gos(&h, boss, HEROIC_BEAKER).len(), 1);
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(HEROIC_BEAKER)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "actual area enemy damage at victim"
    );
    let after = *h.app.world().get::<Position>(player).unwrap();
    assert!(
        knocked && after != before,
        "actual Knockback98 launches the victim"
    );

    let (mut h, boss, _) = maker_copy(1);
    let player = h.player();
    assert!(matches!(
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            boss,
            NORMAL_BEAKER,
            Some(player),
            false
        ),
        Err(crate::spell_cast::CastFailure::UnknownSpell)
    ));
    h.advance(1.0);
    assert!(spell_gos(&h, boss, NORMAL_BEAKER).is_empty());
}
