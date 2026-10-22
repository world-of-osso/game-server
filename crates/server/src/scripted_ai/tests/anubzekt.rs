//! LittleWigs2777a9d AraKara/Anubzekt.lua; actual source215405, native70448.
use super::*;

const ANUBZEKT: u32 = 215405;
const MAP: u32 = 2660;
const BIT: u32 = 1;
const IMPALE: u32 = 435012;
const INFESTATION: u32 = 433740;

fn anubzekt_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ANUBZEKT, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn completed_casts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
#[ignore = "native70448 qualification before local combat port"]
fn encounter_coverage_anubzekt_native_infestation_periodic_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, INFESTATION, Some(boss), false)
            .expect("ordinary native433740 area aura");
        h.advance(2.0);
        assert_eq!(completed_casts(&h, boss, INFESTATION), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(INFESTATION)
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "actual500ms periodic damage difficulty{difficulty}"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
    }
}

#[test]
#[ignore = "native RED: current Lua4.6s Impale, no combat script yet"]
fn encounter_coverage_anubzekt_first_impale_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        h.engage(boss);
        h.advance(4.4);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<CastState>(boss)
            .expect("source4.6s ordinary Impale request");
        assert_eq!(cast.spell_id, IMPALE);
        assert_eq!(
            cast.duration, 4.0,
            "actual Normal/Follower native cast time"
        );
        h.advance(4.1);
        assert_eq!(completed_casts(&h, boss, IMPALE), 1);
    }
}
