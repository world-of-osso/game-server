//! TC a352b1fa boss_leymor.cpp: natural Stomp finish and once-effect Brand child.
use super::azure_combat::{
    assert_first_fissure_and_living_victim, count_self_spell_gos, prepare_leymor_for_later_clock,
};
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;

const STOMP: u32 = 374720;
const STOMP_DAMAGE: u32 = 374731;
const BRAND: u32 = 374567;
const BRAND_CHILD: u32 = 374582;

fn prepare_running_stomp() -> (Harness, Entity) {
    let (mut h, leymor) = prepare_leymor_for_later_clock();
    h.advance(39.2); // Six fixture seconds plus39.2 reaches actual45.2s.
    assert_first_fissure_and_living_victim(&mut h, leymor);
    let cast = h.app.world().get::<CastState>(leymor).unwrap();
    assert_eq!(cast.spell_id, STOMP);
    assert_eq!(cast.cast_type, shared::casting::CastType::Channel);
    assert_eq!(count_self_spell_gos(&h, leymor, STOMP), 1);
    (h, leymor)
}

#[test]
fn azure_leymor_child_stomp_natural_channel_finish_deals_native_damage_once() {
    let (mut h, leymor) = prepare_running_stomp();
    assert!(
        !h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(STOMP_DAMAGE))
    );
    let cast = h.app.world().get::<CastState>(leymor).unwrap();
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert!(h.app.world().get::<CastState>(leymor).is_none());
    let damage: Vec<_> = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| event.spell_id == Some(STOMP_DAMAGE))
        .collect();
    assert_eq!(damage.len(), 1);
    assert_eq!(damage[0].kind, CombatLogKind::Damage);
    assert_eq!(damage[0].source, Some(leymor.to_bits()));
    assert_eq!(damage[0].target, Some(h.player().to_bits()));
    assert!(damage[0].amount > 0);
}

#[test]
fn azure_leymor_child_stomp_cancelled_channel_never_deals_finish_damage() {
    let (mut h, leymor) = prepare_running_stomp();
    crate::spell_cast::cancel_cast(h.app.world_mut(), leymor);
    h.advance(3.0);
    assert!(h.app.world().get::<CastState>(leymor).is_none());
    assert!(
        !h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(STOMP_DAMAGE))
    );
    assert_eq!(count_self_spell_gos(&h, leymor, STOMP), 1);
}

#[test]
fn azure_leymor_child_brand_emits_one_native_field_for_two_real_aura_recipients() {
    let (mut h, leymor) = prepare_leymor_for_later_clock();
    h.advance(24.8);
    assert_first_fissure_and_living_victim(&mut h, leymor);
    // Reuse the second existing fixture player at the first player's actual pose.
    // This is cast-fixture setup, not ordinary admission or navigation proof.
    let other = h.players[1].0;
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    let position = *h.app.world().get::<Position>(h.player()).unwrap();
    assert!(h.app.world().get::<Health>(other).unwrap().current > 0.0);
    h.app.world_mut().entity_mut(other).insert((map, position));
    h.advance(0.3);
    let cast = h.app.world().get::<CastState>(leymor).unwrap();
    assert_eq!(cast.spell_id, BRAND);
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert_eq!(count_self_spell_gos(&h, leymor, BRAND), 1);
    assert!(h.has_aura(h.player(), BRAND));
    assert!(h.has_aura(other, BRAND));
    let fields: Vec<_> = h
        .app
        .world_mut()
        .query::<(&SpellAreaTrigger, &InMap)>()
        .iter(h.app.world())
        .filter(|(field, _)| field.spell_id == BRAND_CHILD)
        .map(|(field, copy)| (field.caster, *copy))
        .collect();
    assert_eq!(fields, vec![(leymor, map)]);
    for index in 0..2 {
        let damage: Vec<_> = h
            .inbox(index)
            .log
            .iter()
            .filter(|event| event.spell_id == Some(BRAND_CHILD))
            .collect();
        assert_eq!(damage.len(), 1, "one native child damage per recipient");
        assert_eq!(damage[0].kind, CombatLogKind::Damage);
        assert_eq!(damage[0].source, Some(leymor.to_bits()));
        assert_eq!(damage[0].target, Some(h.players[index].0.to_bits()));
        assert!(damage[0].amount > 0);
    }
    // Sample after the native750ms lifetime without overriding its spell data.
    h.advance(0.6);
    let child_remains = h
        .app
        .world_mut()
        .query::<&SpellAreaTrigger>()
        .iter(h.app.world())
        .any(|field| field.spell_id == BRAND_CHILD);
    assert!(!child_remains, "native child field expires after750ms");
    // No AT action, movement force, knockback or client-pixel claim.
}
