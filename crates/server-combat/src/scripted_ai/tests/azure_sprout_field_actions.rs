//! Native aura395 scale and CAST/ENEMY action boundaries, not full Azure acceptance.
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;
use std::time::Duration;

const SPROUT_AURA: u32 = 374161;
const SPROUT_DAMAGE: u32 = 374523;
const EXTRA_INITIAL_PERIOD: u32 = 0x200;

fn isolate_native_sprout(h: &mut Harness) -> (Entity, Entity) {
    let children = h.creatures_of(190509);
    assert_eq!(children.len(), 14, "ordinary native summons prerequisite");
    for &child in &children {
        assert!(h.has_aura(child, SPROUT_AURA));
        crate::auras::remove_auras(
            h.app.world_mut(),
            child,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == SPROUT_AURA,
        );
    }
    let child = children[0];
    crate::spell_cast::cast_triggered(h.app.world_mut(), child, SPROUT_AURA, Some(child), 1);
    assert!(h.has_aura(child, SPROUT_AURA));
    (child, children[1])
}

fn find_field(h: &mut Harness, child: Entity) -> Entity {
    let fields: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &SpellAreaTrigger)>()
        .iter(h.app.world())
        .filter(|(_, field)| field.spell_id == SPROUT_AURA && field.caster == child)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(
        fields.len(),
        1,
        "native self-aura must create an actual field"
    );
    fields[0]
}

fn advance_field_clock(h: &mut Harness, milliseconds: u64) {
    // Scale/expiry clock boundary only: do not simulate five minutes of boss events.
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(Duration::from_millis(milliseconds));
    h.app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn azure_aura395_native_scale_reaches_authored_curve_points_before_aura_expiry() {
    let (mut h, _leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let (child, _) = isolate_native_sprout(&mut h);
    let field = find_field(&mut h, child);
    assert_eq!(
        h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius,
        1.25
    );
    advance_field_clock(&mut h, 6_000);
    let grown = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
    assert!(
        (grown - 5.0).abs() < 0.0001,
        "native six-second radius: {grown}"
    );
    advance_field_clock(&mut h, 293_950);
    let near_expiry = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
    // Actual33927 Type0: (0,.25),(.0199999996,1),(1,.962566733).
    let progress = 299_950.0_f32 / 300_000.0;
    let expected = 5.0 * (1.0 + (progress - 0.02) / 0.98 * (0.962566733 - 1.0));
    assert!((near_expiry - expected).abs() < 0.0001);
    advance_field_clock(&mut h, 50);
    assert!(!h.has_aura(child, SPROUT_AURA));
    assert!(
        h.app.world().get_entity(field).is_err(),
        "exact300s aura lifetime"
    );
}

fn count_field_ticks(h: &Harness, caster: Entity, target: Entity) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| event.spell_id == Some(SPROUT_DAMAGE))
        .filter(|event| event.kind == CombatLogKind::Damage)
        .filter(|event| {
            event.source == Some(caster.to_bits()) && event.target == Some(target.to_bits())
        })
        .count()
}

#[test]
fn azure_aura395_native_enemy_fringe_ticks_periodically_and_exit_preserves_other_caster() {
    let (mut h, _leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let (child, other_caster) = isolate_native_sprout(&mut h);
    let field = find_field(&mut h, child);
    let player = h.player();
    let native_flags = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(SPROUT_DAMAGE)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .attributes[5];
    assert_ne!(
        native_flags & EXTRA_INITIAL_PERIOD,
        0,
        "native374523 ticks on apply"
    );
    let center = *h.app.world().get::<Position>(field).unwrap();
    let radius = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), player)
        .expect("source player combat reach");
    h.app.world_mut().entity_mut(player).insert(Position {
        x: center.x + radius + reach * 0.5,
        ..center
    });
    h.players[0].2.log.clear();
    h.advance(0.05);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(player)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPROUT_DAMAGE && aura.caster == child);
    assert!(
        aura.is_some(),
        "source combat reach includes an enemy beyond the visual radius"
    );
    assert_eq!(
        count_field_ticks(&h, child, player),
        1,
        "native ExtraInitialPeriod supplies one initial periodic tick, not an extra hit"
    );
    h.advance(0.9);
    assert_eq!(
        count_field_ticks(&h, child, player),
        1,
        "native1000ms period not yet due; entry must not reapply the aura"
    );
    h.advance(0.15);
    assert_eq!(count_field_ticks(&h, child, player), 2);
    crate::spell_cast::cast_triggered(
        h.app.world_mut(),
        other_caster,
        SPROUT_DAMAGE,
        Some(player),
        1,
    );
    h.app.world_mut().entity_mut(player).insert(Position {
        x: center.x + radius + reach + 10.0,
        ..center
    });
    h.advance(0.05);
    let damage_auras: Vec<_> = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(player)
        .unwrap()
        .iter()
        .filter(|aura| aura.spell_id == SPROUT_DAMAGE)
        .map(|aura| aura.caster)
        .collect();
    assert!(!damage_auras.contains(&child));
    assert!(
        damage_auras.contains(&other_caster),
        "exit removes only the field caster's aura"
    );
    h.advance(1.1);
    assert_eq!(
        count_field_ticks(&h, child, player),
        2,
        "removed aura produces no future ticks"
    );
    assert!(count_field_ticks(&h, other_caster, player) > 0);
}

#[test]
fn azure_aura395_native_field_excludes_self_ally_and_other_copy_with_a_real_enemy_control() {
    let (mut h, _leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let (child, ally) = isolate_native_sprout(&mut h);
    let field = find_field(&mut h, child);
    let center = *h.app.world().get::<Position>(field).unwrap();
    let map = *h.app.world().get::<InMap>(child).unwrap();
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (x, y, z) = crate::terrain_height::bevy_to_wow(center.x, center.y, center.z);
    let (remote, connection) = spawn_player(&mut h.app, other_map, [x, y, z, 0.0], 2);
    h.players.push((remote, connection, Inbox::default()));
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(center);
    h.app.world_mut().entity_mut(ally).insert(center);
    h.advance(0.1);
    assert!(
        h.has_aura(player, SPROUT_DAMAGE),
        "actual same-copy hostile control must be affected"
    );
    assert!(
        !h.has_aura(child, SPROUT_DAMAGE),
        "self is not an enemy action target"
    );
    assert!(
        !h.has_aura(ally, SPROUT_DAMAGE),
        "same-copy ally is not an enemy action target"
    );
    assert!(
        !h.has_aura(remote, SPROUT_DAMAGE),
        "same position in another copy is excluded"
    );
    assert!(h.app.world().get_entity(field).is_ok());
}
