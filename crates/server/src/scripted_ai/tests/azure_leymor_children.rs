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

const FISSURE: u32 = 386660;
const FISSURE_SELECTOR: u32 = 394154;
const SPROUT: u32 = 190509;
const ARCANE_POWER: u32 = 374736;

fn aim_at_nearest_native_sprout(h: &mut Harness, leymor: Entity, children: &[Entity]) -> Entity {
    let origin = *h.app.world().get::<Position>(leymor).unwrap();
    let distance = |child| {
        let pos = h.app.world().get::<Position>(child).unwrap();
        (pos.x - origin.x).hypot(pos.z - origin.z)
    };
    let nearest = children
        .iter()
        .copied()
        .min_by(|left, right| distance(*left).total_cmp(&distance(*right)))
        .unwrap();
    let position = *h.app.world().get::<Position>(nearest).unwrap();
    let offset = Vec3::new(position.x - origin.x, 0.0, origin.z - position.z);
    let selector = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(FISSURE_SELECTOR)
        .unwrap();
    let radius = selector.effects[0].radius[0].unwrap().radius;
    assert!(offset.length() > 0.0 && offset.length() < radius);
    // Aim the controlled fixture at a real native child without moving any Sprout.
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(position);
    h.app.world_mut().get_mut::<Rotation>(leymor).unwrap().y =
        offset.z.atan2(offset.x) + std::f32::consts::FRAC_PI_2;
    nearest
}

#[test]
fn azure_leymor_fissure_completion_kills_native_sprouts_and_consumes_owner_stacks() {
    let (mut h, leymor) = prepare_leymor_for_later_clock();
    h.advance(13.8);
    let children = h.creatures_of(SPROUT);
    assert_eq!(children.len(), 14);
    let nearest = aim_at_nearest_native_sprout(&mut h, leymor, &children);
    let player = h.player();
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        14
    );
    assert!(h.app.world().get::<Health>(player).unwrap().current > 0.0);
    assert_eq!(h.casts(0, leymor, FISSURE), 0);
    h.advance(0.3);
    let cast = h.app.world().get::<CastState>(leymor).unwrap();
    assert_eq!(cast.spell_id, FISSURE);
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert_first_fissure_and_living_victim(&mut h, leymor);
    assert!(
        !crate::spell_cast::targets::is_alive(h.app.world(), nearest),
        "ordinary Fissure394154 hit kills the actual qualifying native Sprout"
    );
    let dead = children
        .iter()
        .filter(|&&child| !crate::spell_cast::targets::is_alive(h.app.world(), child))
        .count();
    assert!(dead > 0 && dead < children.len());
    let remaining = children.len() as u32 - dead as u32;
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        remaining,
        "exactly one owner-stack decrement per actual hit/death"
    );
    h.advance(0.2);
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        remaining
    );
}

#[test]
fn azure_leymor_fissure_cancelled_cast_keeps_all_native_sprouts_and_stacks() {
    let (mut h, leymor) = prepare_leymor_for_later_clock();
    h.advance(13.8);
    let children = h.creatures_of(SPROUT);
    assert_eq!(children.len(), 14);
    aim_at_nearest_native_sprout(&mut h, leymor, &children);
    h.advance(0.3);
    let cast = h.app.world().get::<CastState>(leymor).unwrap();
    assert_eq!(cast.spell_id, FISSURE);
    let remaining = cast.duration - cast.elapsed;
    crate::spell_cast::cancel_cast(h.app.world_mut(), leymor);
    h.advance(remaining + 0.2);
    assert_eq!(h.casts(0, leymor, FISSURE), 0);
    assert!(
        children
            .into_iter()
            .all(|child| crate::spell_cast::targets::is_alive(h.app.world(), child))
    );
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        14
    );
}

#[test]
fn azure_leymor_fissure_selector_uses_caster_front_entry_and_copy_not_destination() {
    let (mut h, leymor) = prepare_leymor_for_later_clock();
    let children = h.creatures_of(SPROUT);
    assert_eq!(children.len(), 14);
    let (front, behind, remote) = (children[0], children[1], children[2]);
    let origin = *h.app.world().get::<Position>(leymor).unwrap();
    let front_pose = Position {
        x: origin.x + 10.0,
        ..origin
    };
    let back_pose = Position {
        x: origin.x - 10.0,
        ..origin
    };
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    let remote_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    // Controlled selection fixture only; ordinary-parent proof above leaves Sprouts unmoved.
    h.app.world_mut().entity_mut(front).insert(front_pose);
    h.app.world_mut().entity_mut(behind).insert(back_pose);
    h.app
        .world_mut()
        .entity_mut(remote)
        .insert((front_pose, remote_map));
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(front_pose);
    h.app.world_mut().get_mut::<Rotation>(leymor).unwrap().y = std::f32::consts::FRAC_PI_2;
    let mut spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(FISSURE_SELECTOR)
        .unwrap()
        .clone();
    spell.cast_destination = Some([back_pose.x, back_pose.y, back_pose.z]);
    assert_eq!(
        spell.misc.as_ref().unwrap().speed,
        0.0,
        "native394154 hits immediately"
    );
    let selected = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        leymor,
        None,
        &spell,
        &spell.effects[0],
        0,
        true,
    )
    .expect("native110 resolves using actual conditions");
    assert!(selected.contains(&front));
    for excluded in [behind, remote, player, leymor] {
        assert!(
            !selected.contains(&excluded),
            "native110 excludes {excluded:?}"
        );
    }
    assert!(selected.iter().all(|child| children.contains(child)));
    crate::spell_cast::cast_triggered_at_destination(
        h.app.world_mut(),
        leymor,
        FISSURE_SELECTOR,
        Vec3::new(back_pose.x, back_pose.y, back_pose.z),
        0,
    );
    // Sample the immediate hit, before generic cleanup unsummons the cross-copy probe
    // whose owner intentionally remains in the original copy (summon.rs:867–881).
    for &child in &selected {
        assert!(!crate::spell_cast::targets::is_alive(h.app.world(), child));
    }
    for survivor in [behind, remote, player] {
        assert!(
            crate::spell_cast::targets::is_alive(h.app.world(), survivor),
            "immediate survivor={survivor:?}, behind={behind:?}, remote={remote:?}, player={player:?}, health={:?}, map={:?}",
            h.app.world().get::<Health>(survivor),
            h.app.world().get::<InMap>(survivor),
        );
    }
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        14 - selected.len() as u32
    );
}

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
