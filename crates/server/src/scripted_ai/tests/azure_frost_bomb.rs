//! TC a352b1fa boss_telash_greywing.cpp: finite Frost Bomb hooks only.
use super::*;
use crate::auras::{RemoveReason, remove_auras};
use crate::spell_cast::area_trigger::SpellAreaTrigger;

const BOMB_CAST: u32 = 386781;
const BOMB_AURA: u32 = 386881;
const BOMB_DAMAGE: u32 = 386910;
const FROZEN_GROUND: u32 = 387149;

fn fixture() -> (Harness, Entity) {
    let (mut h, telash, _) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    // No engagement/AI schedule: exercise the real authored actor's cast boundary.
    h.players[0].2.log.clear();
    (h, telash)
}

fn cast_bomb(h: &mut Harness, telash: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), telash, BOMB_CAST, None, true)
        .expect("native Telash Frost Bomb cast");
    h.advance(0.1);
    if !h.has_aura(h.player(), BOMB_AURA) {
        let world = h.app.world();
        eprintln!(
            "Telash pose={:?} faction={:?}; player pose={:?} faction={:?}; log={:?}",
            world.get::<Position>(telash),
            world.get::<UnitFactionTemplate>(telash),
            world.get::<Position>(h.player()),
            world.get::<UnitFactionTemplate>(h.player()),
            h.players[0].2.log
        );
    }
}

fn ground_fields(h: &mut Harness) -> Vec<(Entity, Position, InMap)> {
    h.app
        .world_mut()
        .query::<(&SpellAreaTrigger, &Position, &InMap)>()
        .iter(h.app.world())
        .filter(|(field, _, _)| field.spell_id == FROZEN_GROUND)
        .map(|(field, position, map)| (field.caster, *position, *map))
        .collect()
}

#[test]
fn azure_frost_bomb_cast_marks_only_selected_players_in_its_copy() {
    let (mut h, telash) = fixture();
    cast_bomb(&mut h, telash);
    assert!(
        h.has_aura(h.player(), BOMB_AURA),
        "source dummy hit must cast386881"
    );
    assert!(
        !h.has_aura(h.players[1].0, BOMB_AURA),
        "other instance is not selected"
    );
    assert!(!h.has_aura(telash, BOMB_AURA));
    assert!(ground_fields(&mut h).is_empty(), "no ground before expiry");
}

#[test]
fn azure_frost_bomb_natural_expiry_casts_ground_and_damage_at_current_target_pose() {
    let (mut h, telash) = fixture();
    cast_bomb(&mut h, telash);
    let player = h.player();
    assert!(h.has_aura(player, BOMB_AURA));
    let expiry = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(player)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == BOMB_AURA)
        .unwrap()
        .expires_at
        .unwrap();
    let now = crate::auras::aura_now(h.app.world());
    h.advance((expiry - now - 0.1).max(0.0) as f32);
    assert!(h.has_aura(player, BOMB_AURA));
    assert!(ground_fields(&mut h).is_empty());
    // Movement after application must not leave the explosion at the old pose.
    h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1.0;
    let position = *h.app.world().get::<Position>(player).unwrap();
    let before = h.app.world().get::<Health>(player).unwrap().current;
    h.advance(0.3);
    assert!(!h.has_aura(player, BOMB_AURA));
    assert_eq!(
        ground_fields(&mut h),
        vec![(
            telash,
            position,
            *h.app.world().get::<InMap>(player).unwrap()
        )]
    );
    assert!(
        h.app.world().get::<Health>(player).unwrap().current < before,
        "native386910 damage"
    );
    for spell in [FROZEN_GROUND, BOMB_DAMAGE] {
        assert!(
            h.players[0].2.log.iter().any(|e| e.spell_id == Some(spell)
                && e.kind == CombatLogKind::CastSuccess
                && e.source == Some(telash.to_bits())),
            "missing source child{spell}"
        );
    }
}

#[test]
fn azure_frost_bomb_nonexpiry_removal_never_explodes() {
    for reason in [
        RemoveReason::Cancelled,
        RemoveReason::Dispelled,
        RemoveReason::Death,
        RemoveReason::Evade,
    ] {
        let (mut h, telash) = fixture();
        cast_bomb(&mut h, telash);
        let player = h.player();
        assert!(h.has_aura(player, BOMB_AURA));
        let removed = remove_auras(h.app.world_mut(), player, reason, |a| {
            a.spell_id == BOMB_AURA
        });
        assert_eq!(removed.len(), 1);
        h.advance(9.0);
        assert!(
            ground_fields(&mut h).is_empty(),
            "unexpected ground on{reason:?}"
        );
        assert!(
            !h.players[0]
                .2
                .log
                .iter()
                .any(|e| e
                    .spell_id
                    .is_some_and(|id| [FROZEN_GROUND, BOMB_DAMAGE].contains(&id))),
            "unexpected child on{reason:?}"
        );
    }
}

#[test]
fn azure_frost_bomb_missing_caster_expiry_does_not_cast_children() {
    let (mut h, telash) = fixture();
    cast_bomb(&mut h, telash);
    assert!(h.has_aura(h.player(), BOMB_AURA));
    h.app.world_mut().entity_mut(telash).despawn();
    h.advance(9.0);
    assert!(!h.has_aura(h.player(), BOMB_AURA));
    assert!(ground_fields(&mut h).is_empty());
    assert!(!h.players[0].2.log.iter().any(|e| {
        e.spell_id
            .is_some_and(|id| [FROZEN_GROUND, BOMB_DAMAGE].contains(&id))
    }));
}
