//! Current Retail LittleWigs2777a9d Crawth; source immunity/ingress remains open.
use super::*;

const CRAWTH: u32 = 191736;
const MAP: u32 = 2526;
const BIT: u32 = 2;
const PECK: u32 = 376997;

fn crawth_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(CRAWTH, MAP, BIT, difficulty, None);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & 0x300,
        0x300,
        "source immunity retained; outgoing-cast proof only"
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

/// TC a352b1fa `WorldObject::IsValidAttackTarget`: the source's own IMMUNE_TO_PC
/// rejects a player target; Peck lacks SPELL_ATTR8_CAN_ATTACK_IMMUNE_PC.
#[test]
fn encounter_coverage_crawth_source_pc_immunity_rejects_native_peck() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = crawth_copy(difficulty);
        let player = h.player();
        assert_eq!(
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, PECK, Some(player), false),
            Err(crate::spell_cast::CastFailure::InvalidTarget),
            "difficulty{difficulty}"
        );
        h.advance(3.0);
        assert_eq!(cast_count(&h, boss, PECK), 0);
        assert!(!h.has_aura(player, PECK));
    }
}

#[test]
#[ignore = "CRAWTH_PC_IMMUNITY_INGRESS_UNSOURCED: source IMMUNE_TO_PC rejects Peck until a sourced unlock"]
fn encounter_coverage_crawth_native_peck_damage_qualifies() {
    for difficulty in [1, 205] {
        let _rng = crate::random::seed_scope(u64::from(PECK) + u64::from(difficulty));
        let (mut h, boss, _) = crawth_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, PECK, Some(player), false)
            .expect("ordinary native376997");
        h.advance(3.0);
        assert_eq!(cast_count(&h, boss, PECK), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(PECK)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "native Peck damage difficulty{difficulty}"
        );
        assert!(h.has_aura(player, PECK));
        assert!(!h.has_aura(h.players[1].0, PECK));
    }
}

#[test]
#[ignore = "CRAWTH_PC_IMMUNITY_INGRESS_UNSOURCED: native threat engagement blocked; no port claimed"]
fn encounter_coverage_crawth_first_peck_uses_current_retail_clock() {
    let (mut h, boss, _) = crawth_copy(205);
    h.engage(boss);
    h.advance(8.0);
    assert_eq!(
        cast_count(&h, boss, PECK),
        1,
        "current Retail5s first Peck, not legacy3.7s"
    );
}
