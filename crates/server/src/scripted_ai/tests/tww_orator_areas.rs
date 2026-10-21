//! Native source areas, not substituted radius-only ground circles.
//! Controlled participant poses exercise the real authored vertical disk and
//! growing sphere; they do not certify navigation or naturally reached heights.
use super::tww_orator::{cast_count, fixture, pull};
use super::*;

const CHAINS_PERIODIC: u32 = 440310;
const INDOCTRINATION: u32 = 434829;
const LINGERING_DAMAGE: u32 = 434926;

fn chains(difficulty: u32) {
    let (mut h, boss, map) = fixture(difficulty);
    pull(&mut h, boss);
    h.advance(0.1);
    assert!(h.has_aura(boss, 434691), "real source engage parent");
    let player = h.player();
    let other = h.players[1].0;
    let center = *h.app.world().get::<Position>(boss).unwrap();
    let ring = Position {
        x: center.x + 20.0,
        y: center.y + 20.0,
        z: center.z,
    };
    h.app.world_mut().entity_mut(player).insert(ring);
    h.app.world_mut().entity_mut(other).insert(ring);
    h.advance(0.15);
    assert!(
        h.has_aura(player, CHAINS_PERIODIC),
        "native31997 disk enter must apply440310"
    );
    assert!(
        !h.has_aura(other, CHAINS_PERIODIC),
        "same pose in another copy is not selected"
    );
    h.advance(1.1);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(434710)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "native4403101000ms tick must land source434710 damage"
    );
    // Move outside the80-yard cylinder while retaining its vertical band.
    h.app.world_mut().get_mut::<Position>(player).unwrap().x = center.x + 100.0;
    h.advance(0.1);
    assert!(
        !h.has_aura(player, CHAINS_PERIODIC),
        "source disk exit removes440310"
    );
    assert_eq!(h.app.world().get::<InMap>(boss), Some(&map));
}

fn lingering(difficulty: u32) {
    let (mut h, boss, _) = fixture(difficulty);
    pull(&mut h, boss);
    for _ in 0..900 {
        h.advance(0.1);
        if cast_count(&h, boss, INDOCTRINATION) > 0 {
            break;
        }
    }
    assert_eq!(
        cast_count(&h, boss, INDOCTRINATION),
        1,
        "native source phase prerequisite"
    );
    h.advance(4.1);
    assert!(!h.has_aura(boss, INDOCTRINATION));
    assert_eq!(
        cast_count(&h, boss, 434923),
        1,
        "source removal creates real parent area"
    );
    let player = h.player();
    let other = h.players[1].0;
    let center = *h.app.world().get::<Position>(boss).unwrap();
    let edge = Position {
        x: center.x + 10.0,
        ..center
    };
    h.app.world_mut().entity_mut(player).insert(edge);
    h.app.world_mut().entity_mut(other).insert(edge);
    h.advance(0.1);
    assert!(
        !h.has_aura(player, LINGERING_DAMAGE),
        "source sphere has not yet grown to10yd"
    );
    h.advance(3.5);
    assert!(
        h.has_aura(player, LINGERING_DAMAGE),
        "source6000ms1→18 curve reaches actual player"
    );
    assert!(!h.has_aura(other, LINGERING_DAMAGE));
    let hp = h.health(player).current;
    h.advance(1.1);
    assert!(
        h.health(player).current < hp,
        "native periodic434926 actual damage"
    );
    h.app.world_mut().get_mut::<Position>(player).unwrap().x = center.x + 30.0;
    h.advance(0.1);
    assert!(
        !h.has_aura(player, LINGERING_DAMAGE),
        "source exit removes434926"
    );
}

#[test]
fn encounter_coverage_tww_orator_normal_chains_native_disk_entry_damage_exit() {
    chains(1);
}
#[test]
fn encounter_coverage_tww_orator_follower205_chains_native_disk_entry_damage_exit() {
    chains(205);
}
#[test]
fn encounter_coverage_tww_orator_normal_lingering_native_growth_damage_exit() {
    lingering(1);
}
#[test]
fn encounter_coverage_tww_orator_follower205_lingering_native_growth_damage_exit() {
    lingering(205);
}
