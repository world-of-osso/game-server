//! LittleWigs2777a9d VoidSpeakerEirich.lua; unchanged source213119 flags.
use super::*;

const EIRICH: u32 = 213119;
const MAP: u32 = 2652;
const BIT: u32 = 2;
const UNBRIDLED_VOID: u32 = 427869;
const SOURCE_FLAGS: u32 = 33_555_008;

#[test]
#[ignore = "native RED: Eirich source ingress and first Void clock unqualified"]
fn encounter_coverage_eirich_source_ingress_and_first_void_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = bound_boss_at_difficulty(EIRICH, MAP, BIT, difficulty, None);
        assert_eq!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0,
            SOURCE_FLAGS
        );
        assert_eq!(SOURCE_FLAGS & 0x100, 0, "actual source is not PC immune");
        assert!(
            h.app
                .world()
                .get::<shared::components::ThreatTable>(boss)
                .is_some(),
            "ordinary source actor must retain native threat ingress, no forged table"
        );
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(boss).insert(visibility);
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        let player = h.player();
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(7.1);
        assert!(
            h.app
                .world()
                .get::<CombatStatus>(boss)
                .is_some_and(|status| status.0),
            "ordinary threat ingress must engage without a fabricated unlock"
        );
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .is_none()
        );
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<shared::casting::CastState>(boss)
            .expect("source Unbridled Void7.3s ordinary native cast");
        assert_eq!(cast.spell_id, UNBRIDLED_VOID, "difficulty{difficulty}");
        assert_eq!(cast.duration, 4.5, "actual70448 native cast time");
        assert_eq!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0 & !0x80000,
            SOURCE_FLAGS,
            "only native IN_COMBAT flag may differ"
        );
    }
}
