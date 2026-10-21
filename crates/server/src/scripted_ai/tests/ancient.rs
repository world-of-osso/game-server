//! Current Retail LittleWigs2777a9d; outgoing combat only, source immunity retained.
use super::*;

const ANCIENT: u32 = 196482;
const MAP: u32 = 2526;
const BIT: u32 = 1;
const BARKBREAKER: u32 = 388544;

fn ancient_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ANCIENT, MAP, BIT, difficulty, None);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & 0x300,
        0x300,
        "source PC/NPC immunity remains: unlock gap, not a native clear"
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_ancient_native_barkbreaker_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = ancient_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, BARKBREAKER, Some(player), false)
            .expect("ordinary native388544");
        h.advance(3.0);
        assert_eq!(cast_count(&h, boss, BARKBREAKER), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(BARKBREAKER)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "native Barkbreaker damage difficulty{difficulty}"
        );
        assert!(h.has_aura(player, BARKBREAKER));
        assert!(!h.has_aura(h.players[1].0, BARKBREAKER));
    }
}

#[test]
#[ignore = "RED: current Retail Ancient Lua-clock port absent"]
fn encounter_coverage_ancient_first_barkbreaker_uses_current_retail_clock() {
    let (mut h, boss, _) = ancient_copy(205);
    h.engage(boss);
    h.advance(12.0);
    assert_eq!(
        cast_count(&h, boss, BARKBREAKER),
        1,
        "current Retail9s initial Barkbreaker"
    );
}
