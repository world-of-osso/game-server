//! Real ordinary phase, Rune entry and native8s cast; no manual aura/AT hooks.
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;
use std::collections::HashSet;

const ABSOLUTE_ZERO_DAMAGE: u32 = 388008;
const RUNE_AURA: u32 = 388065;
const RUNE_SHIELD: u32 = 390201;
const RUNE_PROTECTION: u32 = 388072;
const INACTIVE_RUNE: u32 = 390238;

fn assert_strip_source(h: &Harness) {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(ABSOLUTE_ZERO_DAMAGE)
        .unwrap();
    let effects: Vec<_> = spell
        .effects
        .iter()
        .map(|effect| (effect.index, effect.effect, effect.implicit_targets))
        .collect();
    assert_eq!(effects, vec![(0, 2, [22, 15]), (1, 3, [22, 7])]);
    assert_eq!(
        spell.misc.as_ref().unwrap().cast_time.unwrap().base_ms,
        8000
    );
    let connection =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut statement = connection.prepare(
        "SELECT SourceGroup,ConditionTypeOrReference,ConditionValue1,ConditionValue2 FROM tdb_conditions WHERE SourceTypeOrReferenceId=13 AND SourceEntry=388008 ORDER BY SourceGroup",
    ).unwrap();
    let conditions: Vec<(u32, u32, u32, u32)> = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        conditions,
        vec![(1, 31, 4, 0), (2, 31, 3, 197204)],
        "effect0 hits Players; effect1 hits native Runes"
    );
}

fn completed_zero_casts(h: &Harness, telash: Entity) -> Vec<shared::protocol::SpellGo> {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == telash.to_bits() && go.spell_id == ABSOLUTE_ZERO_DAMAGE)
        .cloned()
        .collect()
}

fn owned_fields(h: &mut Harness, rune: Entity) -> Vec<Entity> {
    h.app
        .world_mut()
        .query::<(Entity, &SpellAreaTrigger)>()
        .iter(h.app.world())
        .filter(|(_, field)| field.spell_id == RUNE_AURA && field.caster == rune)
        .map(|(entity, _)| entity)
        .collect()
}

fn occupied_phase_fixture(difficulty: u32) -> (Harness, Entity, [Entity; 3]) {
    let (mut h, telash) = super::azure_frost_bomb::fixture_at_difficulty(difficulty);
    assert_strip_source(&h);
    super::azure_absolute_zero::complete_ordinary_rune_activation_phase(&mut h, telash);
    let runes: [Entity; 3] = h
        .creatures_of(197204)
        .try_into()
        .expect("native group0 three Runes");
    for rune in runes {
        assert!(
            h.has_aura(rune, RUNE_AURA),
            "ordinary390209 activation prerequisite"
        );
        assert_eq!(owned_fields(&mut h, rune).len(), 1);
    }
    let player = h.player();
    let position = *h.app.world().get::<Position>(runes[0]).unwrap();
    h.app.world_mut().entity_mut(player).insert(position);
    h.advance(0.05);
    assert!(
        h.has_aura(runes[0], RUNE_SHIELD),
        "actual Player entry creates390201"
    );
    assert!(
        h.has_aura(player, RUNE_PROTECTION),
        "actual template entry action"
    );
    assert!(
        !h.has_aura(runes[1], RUNE_SHIELD),
        "unoccupied Rune has no manufactured390201"
    );
    (h, telash, runes)
}

fn reach_native_center_cast(h: &mut Harness, telash: Entity) {
    let seconds = super::azure_jump_charge::read_source_jump_seconds(713);
    h.advance(seconds + 0.1);
    let cast = h
        .app
        .world()
        .get::<CastState>(telash)
        .expect("real outbound landing starts388008");
    assert_eq!(cast.spell_id, ABSOLUTE_ZERO_DAMAGE);
    assert_eq!(cast.cast_type, shared::casting::CastType::Normal);
    assert_eq!(cast.duration, 8.0);
    assert!(completed_zero_casts(h, telash).is_empty());
}

fn advance_before_native_hit(h: &mut Harness, telash: Entity) {
    let cast = h.app.world().get::<CastState>(telash).unwrap();
    let remaining = cast.duration - cast.elapsed;
    assert!(remaining > 0.1);
    h.advance(remaining - 0.1);
    assert_eq!(
        h.app.world().get::<CastState>(telash).unwrap().spell_id,
        ABSOLUTE_ZERO_DAMAGE
    );
    assert!(completed_zero_casts(h, telash).is_empty());
}

#[test]
fn azure_rune_native_388008_completion_strips_selected_occupied_rune_after_real_cast() {
    for difficulty in [205, 23] {
        let (mut h, telash, [occupied, unoccupied, other_copy]) =
            occupied_phase_fixture(difficulty);
        let map = *h.app.world().get::<InMap>(h.players[1].0).unwrap();
        h.app.world_mut().entity_mut(other_copy).insert(map);
        h.advance(0.05);
        assert!(h.app.world().get_entity(other_copy).is_ok());
        assert!(
            h.has_aura(other_copy, RUNE_AURA),
            "already-produced source aura in other copy"
        );
        let occupied_fields = owned_fields(&mut h, occupied);
        let unoccupied_fields = owned_fields(&mut h, unoccupied);
        reach_native_center_cast(&mut h, telash);
        advance_before_native_hit(&mut h, telash);
        assert!(
            h.has_aura(occupied, RUNE_AURA),
            "no strip before actual8s completion"
        );
        assert!(h.has_aura(occupied, RUNE_SHIELD));
        assert!(h.has_aura(unoccupied, RUNE_AURA));
        assert!(h.app.world().get_entity(occupied_fields[0]).is_ok());
        let health = h.health(h.player()).current;
        h.advance(0.15);
        let casts = completed_zero_casts(&h, telash);
        assert_eq!(
            casts.len(),
            1,
            "actual native388008 completed before strip RED"
        );
        assert_eq!(
            casts[0].hit_targets.iter().copied().collect::<HashSet<_>>(),
            HashSet::from([
                h.player().to_bits(),
                occupied.to_bits(),
                unoccupied.to_bits()
            ]),
            "source Player damage plus effect1 same-copy Rune hits, not caster/other copy"
        );
        assert!(
            h.health(h.player()).current < health,
            "effect0 still deals real native Player damage"
        );
        assert!(
            h.inbox(0).log.iter().any(|event| {
                event.spell_id == Some(ABSOLUTE_ZERO_DAMAGE)
                    && event.source == Some(telash.to_bits())
                    && event.kind == CombatLogKind::Damage
            }),
            "actual388008 damage event, not an unrelated health drop"
        );
        assert!(
            !h.has_aura(occupied, RUNE_AURA),
            "actual388008 effect1 must strip388065"
        );
        assert!(
            !h.has_aura(occupied, RUNE_SHIELD),
            "same source hit strips390201"
        );
        assert!(!h.has_aura(unoccupied, RUNE_AURA));
        assert!(
            !h.has_aura(h.player(), RUNE_PROTECTION),
            "field removal undoes real entry action"
        );
        assert!(h.app.world().get_entity(occupied_fields[0]).is_err());
        assert!(h.app.world().get_entity(unoccupied_fields[0]).is_err());
        assert!(
            h.has_aura(other_copy, RUNE_AURA),
            "another-copy source aura survives"
        );
        assert_eq!(
            h.has_aura(occupied, INACTIVE_RUNE),
            difficulty == 23,
            "occupied Mythic removal consequence uses original membership"
        );
        assert!(
            !h.has_aura(unoccupied, INACTIVE_RUNE),
            "no inactive grant to empty Rune"
        );
    }
}

#[test]
fn azure_rune_native_388008_cancelled_cast_preserves_occupied_field_and_shield() {
    let (mut h, telash, [occupied, _, _]) = occupied_phase_fixture(205);
    reach_native_center_cast(&mut h, telash);
    h.advance(2.0);
    assert!(h.has_aura(occupied, RUNE_AURA));
    assert!(h.has_aura(occupied, RUNE_SHIELD));
    let fields = owned_fields(&mut h, occupied);
    crate::spell_cast::cancel_cast(h.app.world_mut(), telash);
    assert!(h.app.world().get::<CastState>(telash).is_none());
    h.advance(8.1);
    assert!(
        completed_zero_casts(&h, telash).is_empty(),
        "cancelled cast cannot produce strip hits"
    );
    assert!(h.has_aura(occupied, RUNE_AURA));
    assert!(h.has_aura(occupied, RUNE_SHIELD));
    assert!(h.has_aura(h.player(), RUNE_PROTECTION));
    assert!(h.app.world().get_entity(fields[0]).is_ok());
    assert!(!h.has_aura(occupied, INACTIVE_RUNE));
}
