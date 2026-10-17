//! TC a352b1fa boss_telash_greywing.cpp: finite Frost Bomb hooks only.
use super::*;
use crate::auras::{RemoveReason, remove_auras};
use crate::spell_cast::area_trigger::SpellAreaTrigger;

const BOMB_CAST: u32 = 386781;
const BOMB_AURA: u32 = 386881;
const BOMB_DAMAGE: u32 = 386910;
const FROZEN_GROUND: u32 = 387149;

pub(super) fn fixture() -> (Harness, Entity) {
    let (mut h, telash, _) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    // Native138 projects the destination onto authored map geometry.
    let wdt = h
        .app
        .world()
        .resource::<MapStore>()
        .maps
        .get(&2515)
        .unwrap()
        .wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(2515, wdt)]),
    ));
    let position = *h.app.world().get::<Position>(telash).unwrap();
    let map = *h.app.world().get::<InMap>(telash).unwrap();
    assert!(
        matches!(
            h.app
                .world_mut()
                .resource_mut::<crate::ground::GroundMap>()
                .ground_in(map, Vec3::new(position.x, position.y, position.z)),
            crate::ground::GroundProbe::Supported(_)
        ),
        "native Azure floor fixture must be available"
    );
    // No engagement/AI schedule: exercise the real authored actor's cast boundary.
    h.players[0].2.log.clear();
    (h, telash)
}

fn cast_bomb(h: &mut Harness, telash: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), telash, BOMB_CAST, None, true)
        .expect("native Telash Frost Bomb cast");
    // Native Attr9/0x10 treats Speed=1 as a one-second delay, not yard/second.
    h.advance(0.9);
    assert!(!h.has_aura(h.player(), BOMB_AURA), "missile not landed yet");
    h.advance(0.3);
}

fn apply_bomb_aura(h: &mut Harness, telash: Entity) {
    let player = h.player();
    crate::spell_cast::creature_cast(h.app.world_mut(), telash, BOMB_AURA, Some(player), true)
        .expect("native Frost Bomb aura cast");
    h.advance(0.1);
}

fn project_authored_ground(h: &mut Harness, map: InMap, mut position: Position) -> Position {
    let probe = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(map, Vec3::new(position.x, position.y, position.z));
    let crate::ground::GroundProbe::Supported(height) = probe else {
        panic!("native Azure floor at {position:?}: {probe:?}");
    };
    position.y = height;
    position
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
    apply_bomb_aura(&mut h, telash);
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
    let map = *h.app.world().get::<InMap>(player).unwrap();
    let position = project_authored_ground(&mut h, map, position);
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
        apply_bomb_aura(&mut h, telash);
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
fn azure_frost_bomb_destination_without_unit_uses_authored_caster_ground() {
    let (mut h, telash) = fixture();
    let position = *h.app.world().get::<Position>(telash).unwrap();
    let map = *h.app.world().get::<InMap>(telash).unwrap();
    let expected = project_authored_ground(&mut h, map, position);
    crate::spell_cast::creature_cast(h.app.world_mut(), telash, FROZEN_GROUND, None, true).unwrap();
    h.advance(0.1);
    assert_eq!(ground_fields(&mut h), vec![(telash, expected, map)]);
}

#[test]
fn azure_frost_bomb_requested_destination_precedes_supplied_unit() {
    let (mut h, telash) = fixture();
    let player = h.player();
    let mut position = *h.app.world().get::<Position>(telash).unwrap();
    position.x += 5.0;
    let map = *h.app.world().get::<InMap>(telash).unwrap();
    let expected = project_authored_ground(&mut h, map, position);
    let mut spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(FROZEN_GROUND)
        .unwrap()
        .clone();
    // Runtime cast input only: native effect selectors/radius/duration unchanged.
    spell.cast_destination = Some([position.x, position.y, position.z]);
    h.app.insert_resource(SpellStore::fixture(vec![spell]));
    crate::spell_cast::creature_cast(h.app.world_mut(), telash, FROZEN_GROUND, Some(player), true)
        .unwrap();
    h.advance(0.1);
    assert_eq!(ground_fields(&mut h), vec![(telash, expected, map)]);
}

#[test]
fn azure_frost_bomb_missing_caster_cleanup_does_not_cast_children() {
    let (mut h, telash) = fixture();
    apply_bomb_aura(&mut h, telash);
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
