//! TC a352b1fa boss_telash_greywing.cpp: native power and first engagement boundaries.
use super::*;
use crate::spell_cast::powers::{PowerTypeTable, power_value};
use shared::components::PowerType;

const ICE_POWER: u32 = 389453;
const FROST_BOMB: u32 = 386781;
const FROST_BOMB_AURA: u32 = 386881;
const ICY_DEVASTATOR: u32 = 387151;
const ICY_DEVASTATOR_DAMAGE: u32 = 387152;
const VAULT_RUNE: u32 = 197204;

#[test]
fn azure_telash_native_group_zero_spawns_three_owned_runes_at_authored_positions_once() {
    let connection = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read-only authored Rune group positions");
    let mut statement = connection
        .prepare("SELECT position_x,position_y,position_z FROM tdb_creature_summon_groups WHERE summonerId=186737 AND summonerType=0 AND groupId=0 AND entry=197204")
        .unwrap();
    let expected_positions = statement
        .query_map([], |row| {
            let (x, y, z) =
                crate::terrain_height::wow_to_bevy(row.get(0)?, row.get(1)?, row.get(2)?);
            Ok(Position { x, y, z })
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(expected_positions.len(), 3, "actual group0 prerequisite");
    let (mut h, telash, map) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    assert!(
        h.app
            .world()
            .resource::<WorldData>()
            .templates
            .contains_key(&VAULT_RUNE),
        "actual Rune template prepared by the source producer, not a hand-built actor"
    );
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0)),
        "native initial power prerequisite"
    );
    let runes = h.creatures_of(VAULT_RUNE);
    assert_eq!(
        runes.len(),
        3,
        "JustAppeared must summon its actual source group0"
    );
    let position_key = |position: &Position| {
        (
            position.x.to_bits(),
            position.y.to_bits(),
            position.z.to_bits(),
        )
    };
    let expected: std::collections::HashSet<_> =
        expected_positions.iter().map(position_key).collect();
    let actual: std::collections::HashSet<_> = runes
        .iter()
        .map(|&rune| {
            assert_eq!(h.app.world().get::<InMap>(rune), Some(&map));
            assert_eq!(
                h.app
                    .world()
                    .get::<crate::zones::TempSummon>(rune)
                    .unwrap()
                    .summoner,
                telash,
                "actual source summoner owns each Rune"
            );
            assert!(h.app.world().get::<Health>(rune).unwrap().current > 0.0);
            position_key(h.app.world().get::<Position>(rune).unwrap())
        })
        .collect();
    assert_eq!(
        actual, expected,
        "exact authored positions in the owner's copy"
    );
    let before: std::collections::HashSet<_> = runes.into_iter().collect();
    h.advance(0.1);
    let after: std::collections::HashSet<_> = h.creatures_of(VAULT_RUNE).into_iter().collect();
    assert_eq!(
        after, before,
        "ordinary ticks must not duplicate the startup group"
    );
}

#[test]
fn azure_telash_appears_with_sixty_native_energy() {
    let (mut h, telash, _) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    let energy = h
        .app
        .world()
        .resource::<PowerTypeTable>()
        .get(PowerType::Energy)
        .expect("native Energy data prerequisite");
    assert_eq!(energy.max_base_power, 100);
    // No synthetic powers, runes or alternate actor: observe the spawned native boss.
    h.advance(0.1);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0)),
        "source JustAppeared initializes native primary Energy to60"
    );
}

#[test]
fn azure_telash_native_flag_stops_regen_without_changing_controlled_pools() {
    let (mut h, telash, _) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    h.advance(0.1);
    // Isolate baseline permission from the now-authentic scripted combat aura.
    let removed = crate::auras::remove_auras(
        h.app.world_mut(),
        telash,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ICE_POWER,
    );
    assert_eq!(
        removed.len(),
        1,
        "remove only the actual engagement aura control"
    );
    h.advance(1.0);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0)),
        "authored unit_flags2=0 prevents unauthored baseline regeneration"
    );
    // Control the existing companion resource path, not actual Telash gameplay.
    h.app
        .world_mut()
        .get_mut::<shared::components::UnitFlags>(telash)
        .unwrap()
        .0 |= crate::faction::UNIT_FLAG_PLAYER_CONTROLLED;
    h.advance(0.2);
    let (current, maximum) = power_value(h.app.world(), telash, PowerType::Energy).unwrap();
    assert!(
        current > 60.0,
        "controlled pools retain native player-rate regeneration"
    );
    assert_eq!(
        maximum, 100.0,
        "control does not manufacture a new capacity"
    );
}

#[test]
fn azure_telash_ice_power_ticks_add_two_energy_each_native_second() {
    let (mut h, telash, _) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    h.advance(0.1);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0))
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), telash, ICE_POWER, Some(telash), true)
        .expect("native Ice Power aura cast");
    assert!(
        h.has_aura(telash, ICE_POWER),
        "native aura prerequisite before periodic RED"
    );
    h.advance(0.8);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0)),
        "no baseline or premature periodic power before the native one-second tick"
    );
    h.advance(0.3);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((62.0, 100.0)),
        "source periodic dummy adds two primary Energy at the first native tick"
    );
    h.advance(1.0);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((64.0, 100.0)),
        "source periodic dummy repeats on the next native tick"
    );
}

pub(super) fn assert_living_victim(h: &mut Harness, telash: Entity) {
    let player = h.player();
    assert!(h.app.world().get::<Health>(player).unwrap().current > 0.0);
    assert_eq!(
        (Ctx {
            world: h.app.world_mut(),
            me: telash
        })
        .victim(),
        Some(player),
        "ordinary native engagement has a living actual victim"
    );
}

#[test]
fn azure_telash_engagement_applies_native_ice_power_to_self() {
    let (mut h, telash) = super::azure_frost_bomb::fixture();
    h.engage(telash);
    h.advance(0.1);
    assert_living_victim(&mut h, telash);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0))
    );
    assert!(
        h.has_aura(telash, ICE_POWER),
        "source engagement casts native389453 on self"
    );
    h.advance(1.0);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((62.0, 100.0))
    );
}

#[test]
fn azure_telash_frost_bomb_starts_at_three_point_five_and_lands_native_aura() {
    let (mut h, telash) = super::azure_frost_bomb::fixture();
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(telash).insert(visibility);
    let casts = |h: &Harness| {
        h.inbox(0)
            .spell_gos
            .iter()
            .filter(|go| go.caster == telash.to_bits() && go.spell_id == FROST_BOMB)
            .count()
    };
    h.engage(telash);
    h.advance(3.3);
    assert_living_victim(&mut h, telash);
    assert_eq!(casts(&h), 0);
    assert!(!h.has_aura(h.player(), FROST_BOMB_AURA));
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(telash)
        .expect("source3.5s Frost Bomb starts its native cast");
    assert_eq!(cast.spell_id, FROST_BOMB);
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    assert_eq!(casts(&h), 1, "one completed ordinary native Frost parent");
    assert!(
        h.inbox(0)
            .spell_gos
            .iter()
            .any(|go| go.caster == telash.to_bits()
                && go.spell_id == FROST_BOMB
                && go.target == Some(telash.to_bits()))
    );
    // The native parent has a one-second missile delay after its real two-second cast.
    h.advance(1.2);
    assert_living_victim(&mut h, telash);
    assert!(
        h.has_aura(h.player(), FROST_BOMB_AURA),
        "native dummy hit applies386881"
    );
}

#[test]
fn azure_telash_icy_devastator_follows_first_frost_reschedule_and_native_damage() {
    let (mut h, telash) = super::azure_frost_bomb::fixture();
    let player = h.player();
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(telash).insert(visibility);
    h.engage(telash);
    // TC firstFrost3.5s reschedules the initial14.5s Devastator10.8s later:14.3s.
    h.advance(14.1);
    assert_living_victim(&mut h, telash);
    assert!(
        h.has_aura(telash, ICE_POWER),
        "ordinary native engagement aura prerequisite"
    );
    assert_eq!(
        h.inbox(0)
            .spell_gos
            .iter()
            .filter(|go| go.caster == telash.to_bits() && go.spell_id == FROST_BOMB)
            .count(),
        1,
        "first ordinary Frost completed before its source rescheduled deadline"
    );
    assert!(
        !h.inbox(0)
            .spell_gos
            .iter()
            .any(|go| go.caster == telash.to_bits() && go.spell_id == ICY_DEVASTATOR)
    );
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(telash)
        .expect("source firstFrost reschedule starts Icy Devastator at14.3s");
    assert_eq!(cast.spell_id, ICY_DEVASTATOR);
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    let channel = h.app.world().get::<CastState>(telash).unwrap();
    assert_eq!(channel.spell_id, ICY_DEVASTATOR);
    assert_eq!(channel.cast_type, shared::casting::CastType::Channel);
    assert_eq!(channel.target, player.to_bits());
    // Keep native channel duration and give its1000ms trigger387152 a real tick.
    h.advance(1.0);
    assert!(
        h.inbox(0)
            .spell_gos
            .iter()
            .any(|go| go.caster == telash.to_bits()
                && go.spell_id == ICY_DEVASTATOR
                && go.target == Some(player.to_bits())),
        "native random-player cast target"
    );
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(ICY_DEVASTATOR_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.source == Some(telash.to_bits())
                && event.target == Some(player.to_bits())
                && event.amount > 0),
        "native periodic trigger387152 damages the selected player from Telash"
    );
}
