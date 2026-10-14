//! Source Silverlaine spell/lifecycle boundary; not a complete fight or native clear.
use super::*;

const SILVERLAINE: u32 = 3887;
const VEIL_OF_SHADOW: u32 = 7068;
const SILVERLAINE_BIT: u32 = 1;

#[test]
fn encounter_coverage_silverlaine_real_source_cast_repeat_and_death_credit() {
    let (mut h, boss, map) = encounter_fixture(
        SILVERLAINE,
        33,
        SILVERLAINE_BIT,
        1,
        None,
        EncounterState::NotStarted,
        false,
    );
    let boss = boss.unwrap();
    let player = h.player();
    // Isolate the source clock from player survival; actor, spell, timer and chance stay real.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.engage(boss);
    h.advance(0.8);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 0);
    assert!(!h.has_aura(player, VEIL_OF_SHADOW));
    h.advance(3.0);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 1);
    assert!(h.has_aura(player, VEIL_OF_SHADOW));
    assert!(!h.has_aura(h.players[1].0, VEIL_OF_SHADOW));
    h.advance(36.0);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 2);
    assert_eq!(h.casts(1, boss, VEIL_OF_SHADOW), 0);
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, SILVERLAINE_BIT),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 1070 && end.success)
    );
    assert!(h.inbox(1).ends.is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << SILVERLAINE_BIT
    );
}
