//! TC a352b1fa Leymor/Telash EnterEvadeMode: DespawnAll, _EnterEvadeMode, _DespawnAtEvade.
use super::*;

const TELASH: u32 = 186737;
const VAULT_RUNE: u32 = 197204;
const SPROUT: u32 = 190509;

fn living(h: &mut Harness, entry: u32) -> Vec<Entity> {
    h.creatures_of(entry)
        .into_iter()
        .filter(|&actor| crate::spell_cast::targets::is_alive(h.app.world(), actor))
        .collect()
}

#[test]
fn azure_telash_evade_despawns_runes_and_respawned_boss_resummons_them() {
    let (mut h, telash, map) = bound_boss_at_difficulty(TELASH, 2515, 2, 205, None);
    let before = living(&mut h, VAULT_RUNE);
    assert_eq!(before.len(), 3, "JustAppeared group0 Runes");
    let player = h.player();
    let position = *h.app.world().get::<Position>(telash).unwrap();
    h.app.world_mut().entity_mut(player).insert(position);
    h.engage(telash);
    h.advance(0.2);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), telash);
    h.advance(0.1);
    assert!(
        living(&mut h, VAULT_RUNE).is_empty(),
        "summons.DespawnAll removes the owned Runes"
    );
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::Fail)
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(telash)
            .is_some(),
        "_DespawnAtEvade retires Telash without a kill"
    );
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 500.0,
        ..position
    });
    h.advance(30.1);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(telash)
            .is_none(),
        "default30s respawn"
    );
    let after = living(&mut h, VAULT_RUNE);
    assert_eq!(after.len(), 3, "reappearance summons group0 again");
    assert!(after.iter().all(|rune| !before.contains(rune)));
}

#[test]
fn azure_leymor_owns_engaged_sprouts_and_evade_despawns_them() {
    let (mut h, leymor) = super::azure_combat::wake_leymor();
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    super::azure_sprouts::prepare_sprout_templates(&mut h);
    super::azure_sprouts::install_azure_ground(&mut h, map);
    h.engage(leymor);
    // Same clock as prepare_leymor_for_later_clock: the 3s cast plus missile flights.
    h.advance(6.0);
    let sprouts = living(&mut h, SPROUT);
    assert_eq!(sprouts.len(), 14, "first native Ley-Line Sprouts");
    // Sprouts are IMMUNE_TO_PC|NPC (unit_flags 0x300): BossAI::JustSummoned's
    // DoZoneInCombat skips them (CanBeginCombat), but summons still owns them.
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), leymor);
    h.advance(0.1);
    assert!(
        living(&mut h, SPROUT).is_empty(),
        "summons.DespawnAll removes the owned Sprouts"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(leymor)
            .is_some(),
        "_DespawnAtEvade retires Leymor without a kill"
    );
}
