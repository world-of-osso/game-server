//! Golden Gryphon 32235 (Retail flying mount, `Mount` 129, MountType 424) against the real
//! spell and mount DB2 tables in data/world.db.

use rusqlite::{Connection, OpenFlags};
use shared::components::{CombatStatus, Mounted, PlayerMotion};
use shared::profession::ProfessionSkillLine;
use shared::protocol::{CancelMountAura, CombatChannel};
use shared::spell_data::CastFailReason;

use super::mount::{
    FLIGHT_STYLE_SKYRIDING, FLIGHT_STYLE_STEADY, FlightStyle, SKILL_RIDING, SKYRIDING_BASICS,
    load_mount_store, set_flight_style,
};
use super::*;
use crate::profession::PlayerProfessions;
use crate::spell_cast::test_harness::{Harness, harness, world_db};

/// Golden Gryphon: MOUNTED 78, MiscValue 18360 (creature), MiscValueB 248.
const GOLDEN_GRYPHON: u32 = 32235;
/// `MountXDisplay` 200 of Mount 129.
const GOLDEN_GRYPHON_DISPLAY: u32 = 17697;
/// MountCapability 226 / 227 (ground, riding 75 / 150).
const STANDARD_GROUND: u32 = 86457;
const EPIC_GROUND: u32 = 86458;
/// MountCapability 455 / 456 (steady flight, riding 225 / 300, Flight Style: Steady).
const STANDARD_FLYING: u32 = 86459;
const EPIC_FLYING: u32 = 86460;
const LEGENDARY_FLYING: u32 = 86461;
/// MountCapability 494's speed aura: Skyriding (ADV_FLYING 446, mounted speed 100%).
const SKYRIDING: u32 = 406095;

pub(crate) fn rider(riding: u16, flight_style: bool, extra: &[u32]) -> Option<Harness> {
    let data = world_db()?;
    let mut spells = vec![
        GOLDEN_GRYPHON,
        STANDARD_GROUND,
        EPIC_GROUND,
        STANDARD_FLYING,
        EPIC_FLYING,
        LEGENDARY_FLYING,
        SKYRIDING,
        FLIGHT_STYLE_SKYRIDING,
    ];
    spells.extend_from_slice(extra);
    if flight_style {
        spells.push(FLIGHT_STYLE_STEADY);
    }
    let mut h = harness(Vec::new(), &spells, 1)?;
    let conn =
        Connection::open_with_flags(&data.powers_db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app.insert_resource(load_mount_store(&conn).unwrap());
    let step = riding / 75;
    h.app.world_mut().entity_mut(h.caster).insert((
        CombatStatus(false),
        PlayerMotion::default(),
        PlayerProfessions {
            lines: vec![ProfessionSkillLine {
                skill_line: SKILL_RIDING,
                step,
                rank: riding,
                max_rank: riding,
            }],
            spells: Vec::new(),
        },
    ));
    Some(h)
}

/// Cast the 1.5 s mount and let it finish.
pub(crate) fn mount(h: &mut Harness) {
    h.cast_and_deliver(GOLDEN_GRYPHON, None);
    h.advance(32);
}

fn has(h: &Harness, spell_id: u32) -> bool {
    has_aura(h.app.world(), h.caster, spell_id)
}

fn motion(h: &Harness) -> PlayerMotion {
    *h.app.world().get::<PlayerMotion>(h.caster).unwrap()
}

fn stat_mods(h: &Harness) -> AuraStatMods {
    h.app.world().get::<AuraStatMods>(h.caster).unwrap().clone()
}

fn guild_rider(extra: &[u32]) -> Harness {
    let spells: Vec<_> = [78633].into_iter().chain(extra.iter().copied()).collect();
    let mut h = rider(225, true, &spells).expect("Actual Golden Gryphon and mount catalog");
    // The harness loads only requested native spell records into SpellStore.
    // Keep78633 available there, but derive its knowledge from the guild grant.
    let baseline = h
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .iter()
        .filter(|id| *id != 78633)
        .collect();
    let mut known = crate::class_progression::KnownSpells::fixture(baseline);
    known.set_guild_grants(&[78633]);
    h.app.world_mut().entity_mut(h.caster).insert(known);
    h.advance(3);
    h
}

#[test]
fn guild_mount_up_real_spell_grant_modifies_ground_and_flight_then_revokes() {
    // Active GuildPerkSpells2→78633; SpellEffect68259/68260/102117 are172/211/210.
    // TC Unit.cpp8812–8854: mounted base multiplier × max(stacking, nonstacking).
    let mut h = guild_rider(&[]);
    assert!(
        has(&h, 78633),
        "Real known passive is applied, not a fabricated speed aura"
    );
    assert_eq!(
        stat_mods(&h).movement_speed,
        1.0,
        "Unmounted running is unchanged"
    );
    mount(&mut h);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let mounted = stat_mods(&h);
    assert!(
        (mounted.movement_speed - 2.2).abs() < 1e-6,
        "Actual mounted ground speed: {}",
        mounted.movement_speed
    );
    assert!(
        (mounted.flight_speed - 3.52).abs() < 1e-6,
        "Actual mounted flight speed: {}",
        mounted.flight_speed
    );
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_guild_grants(&[]);
    h.advance(3);
    assert!(!has(&h, 78633));
    let departed = stat_mods(&h);
    assert_eq!(departed.movement_speed, 2.0);
    assert!((departed.flight_speed - 3.2).abs() < 1e-6);
    assert!(
        has(&h, GOLDEN_GRYPHON),
        "Guild departure does not unmount or unlearn class spells"
    );
    cancel_mount(&mut h);
    assert_eq!(stat_mods(&h).movement_speed, 1.0);
}

#[test]
fn guild_mount_up_real_crusader_aura_uses_stronger_bonus_without_stacking() {
    // Actual32223 effects172/211 are20%, stronger than Mount Up's10%.
    let mut h = guild_rider(&[32223]);
    mount(&mut h);
    let swim_without_crusader = stat_mods(&h).swim_speed;
    h.cast_and_deliver(32223, None);
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has(&h, 32223) && has(&h, 78633));
    let combined = stat_mods(&h);
    assert!(
        (combined.movement_speed - 2.4).abs() < 1e-6,
        "{}",
        combined.movement_speed
    );
    assert!(
        (combined.flight_speed - 3.84).abs() < 1e-6,
        "{}",
        combined.flight_speed
    );
    assert_eq!(
        combined.swim_speed, swim_without_crusader,
        "172/211 must not leak into swim speed"
    );
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_guild_grants(&[]);
    h.advance(3);
    assert!(!has(&h, 78633) && has(&h, 32223));
    assert!((stat_mods(&h).movement_speed - 2.4).abs() < 1e-6);
}

#[test]
fn guild_mount_up_real_map_transitions_suppress_and_restore_without_unlearning() {
    use crate::maps::{
        InMap,
        store::{InstanceType, load_map_store},
    };
    let mut h = guild_rider(&[]);
    let data = world_db().expect("Actual installed map catalog");
    let conn =
        Connection::open_with_flags(&data.powers_db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let maps = load_map_store(&conn).unwrap();
    assert_eq!(
        maps.map(489).unwrap().instance_type,
        InstanceType::Battleground
    );
    assert_eq!(maps.map(559).unwrap().instance_type, InstanceType::Arena);
    h.app.insert_resource(maps);
    mount(&mut h);
    for (map_id, eligible) in [(489, false), (559, false), (0, true)] {
        h.app.world_mut().entity_mut(h.caster).insert(InMap {
            map_id,
            instance_id: u32::from(map_id != 0),
        });
        h.advance(3);
        assert!(
            h.app
                .world()
                .get::<crate::class_progression::KnownSpells>(h.caster)
                .unwrap()
                .contains(78633)
        );
        assert_eq!(
            has(&h, 78633),
            eligible,
            "Authored passive eligibility on actual map{map_id}"
        );
        let expected = if eligible { 2.2 } else { 2.0 };
        assert!((stat_mods(&h).movement_speed - expected).abs() < 1e-6);
    }
}

#[test]
fn players_entering_the_world_carry_flight_style_steady() {
    let Some(h) = rider(225, true, &[]) else {
        return;
    };
    assert!(has(&h, FLIGHT_STYLE_STEADY));
}

#[test]
fn mount_special_identity_comes_from_the_authoritative_mount_aura() {
    let mut h = rider(225, true, &[]).expect("authentic Golden Gryphon test catalog");
    mount(&mut h);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let mounted = h.app.world().get::<Mounted>(h.caster).unwrap();
    let wire = serde_json::to_value(mounted).unwrap();
    assert_eq!(
        wire.get("mount_id").and_then(serde_json::Value::as_u64),
        Some(129)
    );
    assert_eq!(mounted.mount_display_id, GOLDEN_GRYPHON_DISPLAY);
}

#[test]
fn expert_riding_mounts_the_golden_gryphon_with_standard_steady_flight() {
    let Some(mut h) = rider(225, true, &[]) else {
        return;
    };
    mount(&mut h);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app.world().get::<Mounted>(h.caster),
        Some(&Mounted {
            mount_id: 129,
            mount_display_id: GOLDEN_GRYPHON_DISPLAY,
            ..Default::default()
        })
    );
    assert!(has(&h, GOLDEN_GRYPHON));
    assert!(has(&h, STANDARD_FLYING));
    assert!(!has(&h, STANDARD_GROUND));
    assert!(motion(&h).contains(PlayerMotion::CAN_FLY));
    // 86459: MOD_INCREASE_MOUNTED_SPEED 100%, MOD_INCREASE_MOUNTED_FLIGHT_SPEED 220%.
    let mods = stat_mods(&h);
    assert_eq!(mods.movement_speed, 2.0);
    assert!(
        (mods.flight_speed - 3.2).abs() < 1e-6,
        "{}",
        mods.flight_speed
    );
}

#[test]
fn riding_skill_picks_the_capability_speed_aura() {
    for (riding, speed_aura, flies) in [
        (75, STANDARD_GROUND, false),
        (150, EPIC_GROUND, false),
        (300, EPIC_FLYING, true),
        (375, LEGENDARY_FLYING, true),
    ] {
        let Some(mut h) = rider(riding, true, &[]) else {
            return;
        };
        mount(&mut h);
        assert!(has(&h, speed_aura), "riding {riding}");
        assert_eq!(
            motion(&h).contains(PlayerMotion::CAN_FLY),
            flies,
            "riding {riding}"
        );
    }
}

#[test]
fn without_flight_style_steady_the_mount_stays_on_the_ground() {
    let Some(mut h) = rider(225, false, &[]) else {
        return;
    };
    mount(&mut h);
    assert!(h.app.world().get::<Mounted>(h.caster).is_some());
    assert!(has(&h, EPIC_GROUND));
    assert!(!has(&h, STANDARD_FLYING));
    assert!(!motion(&h).contains(PlayerMotion::CAN_FLY));
}

#[test]
fn mounting_in_combat_fails() {
    let Some(mut h) = rider(225, true, &[]) else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(true));
    mount(&mut h);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::AffectingCombat);
    assert!(h.app.world().get::<Mounted>(h.caster).is_none());
}

#[test]
fn a_mounted_player_cannot_cast_and_dismounts_with_cancel_mount_aura() {
    let Some(mut h) = rider(225, true, &[]) else {
        return;
    };
    mount(&mut h);
    mount(&mut h);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotMounted);

    h.app
        .world_mut()
        .get_mut::<MessageSender<CancelMountAura>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(CancelMountAura);
    h.advance(3);
    assert!(h.app.world().get::<Mounted>(h.caster).is_none());
    assert!(!has(&h, GOLDEN_GRYPHON));
    assert!(!has(&h, STANDARD_FLYING));
    assert!(!motion(&h).contains(PlayerMotion::CAN_FLY));
    let mods = stat_mods(&h);
    assert_eq!((mods.movement_speed, mods.flight_speed), (1.0, 1.0));
}

fn cancel_mount(h: &mut Harness) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<CancelMountAura>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(CancelMountAura);
    h.advance(3);
}

/// MountCapability 494 (Skyriding, FlightCapability 11) comes first for MountType 424 and
/// needs Skyriding Basics 376777 known and PlayerCondition 96927: NOT Flight Style: Steady.
/// While Steady is on, the same rider gets steady flight; without Skyriding Basics, a
/// rider without Steady stays on the ground (see above).
#[test]
fn switching_the_flight_style_picks_skyriding_or_steady_flight() {
    let Some(mut h) = rider(225, true, &[]) else {
        return;
    };
    mount(&mut h);
    assert!(has(&h, STANDARD_FLYING) && !has(&h, SKYRIDING));
    assert!(!motion(&h).contains(PlayerMotion::CAN_ADV_FLY));
    cancel_mount(&mut h);

    // game-server-admin flight-style <name> skyriding.
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.caster)
        .unwrap()
        .spells
        .push(SKYRIDING_BASICS);
    set_flight_style(h.app.world_mut(), h.caster, FlightStyle::Skyriding);
    assert!(has(&h, FLIGHT_STYLE_SKYRIDING) && !has(&h, FLIGHT_STYLE_STEADY));
    mount(&mut h);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has(&h, SKYRIDING) && !has(&h, STANDARD_FLYING));
    assert!(
        motion(&h).contains(PlayerMotion::CAN_FLY | PlayerMotion::CAN_ADV_FLY),
        "{:?}",
        motion(&h)
    );
    // 406095: MOD_INCREASE_MOUNTED_SPEED 100%.
    assert_eq!(stat_mods(&h).movement_speed, 2.0);

    cancel_mount(&mut h);
    assert!(!motion(&h).contains(PlayerMotion::CAN_FLY));
    assert!(!motion(&h).contains(PlayerMotion::CAN_ADV_FLY));

    set_flight_style(h.app.world_mut(), h.caster, FlightStyle::Steady);
    assert!(has(&h, FLIGHT_STYLE_STEADY) && !has(&h, FLIGHT_STYLE_SKYRIDING));
    mount(&mut h);
    assert!(has(&h, STANDARD_FLYING) && !has(&h, SKYRIDING));
}

/// Switch Flight Style 436854 (the Riding skill line's ability; DUMMY 0 on the caster,
/// spelldesc `$?A404468[Steady Flight][Skyriding]`).
const SWITCH_FLIGHT_STYLE: u32 = 436_854;
/// Switch Flight Style 460002 (icon ability_dragonriding_dynamicflight01) and 460003
/// (ability_dragonriding_staticflight01), DUMMY 0 on the caster.
const SWITCH_TO_SKYRIDING: u32 = 460_002;
const SWITCH_TO_STEADY: u32 = 460_003;
/// The switch's 5 s cast (SpellCastTimes 6) in 50 ms frames, and one more.
const SWITCH_CAST_FRAMES: usize = 101;
const CHARACTER: u64 = 7;

/// An expert rider on Flight Style: Steady who knows Skyriding Basics and the switches,
/// saved as character 7.
fn switcher() -> Option<Harness> {
    let mut h = rider(
        225,
        true,
        &[
            SKYRIDING_BASICS,
            SWITCH_FLIGHT_STYLE,
            SWITCH_TO_SKYRIDING,
            SWITCH_TO_STEADY,
        ],
    )?;
    h.app.insert_resource(crate::persistence::GameDatabase {
        db: crate::persistence::temp_database(),
    });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(CHARACTER));
    Some(h)
}

fn saved_style(h: &Harness) -> Option<u32> {
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    crate::persistence_progression::load_flight_style(db, CHARACTER)
}

fn switch(h: &mut Harness, spell: u32) {
    h.cast_and_deliver(spell, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(SWITCH_CAST_FRAMES);
}

/// Casting Switch Flight Style swaps the Flight Style aura, saves it, and the next mount
/// flies that style: Skyriding (406095, ADV_FLYING) or steady flight (86459).
#[test]
fn switch_flight_style_toggles_the_style_the_next_mount_flies_and_saves_it() {
    let Some(mut h) = switcher() else {
        return;
    };
    switch(&mut h, SWITCH_FLIGHT_STYLE);
    assert!(has(&h, FLIGHT_STYLE_SKYRIDING) && !has(&h, FLIGHT_STYLE_STEADY));
    assert_eq!(saved_style(&h), Some(FLIGHT_STYLE_SKYRIDING));
    mount(&mut h);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has(&h, SKYRIDING) && !has(&h, STANDARD_FLYING));
    assert!(motion(&h).contains(PlayerMotion::CAN_ADV_FLY));
    cancel_mount(&mut h);

    switch(&mut h, SWITCH_FLIGHT_STYLE);
    assert!(has(&h, FLIGHT_STYLE_STEADY) && !has(&h, FLIGHT_STYLE_SKYRIDING));
    assert_eq!(saved_style(&h), Some(FLIGHT_STYLE_STEADY));
    mount(&mut h);
    assert!(has(&h, STANDARD_FLYING) && !has(&h, SKYRIDING));
    assert!(!motion(&h).contains(PlayerMotion::CAN_ADV_FLY));
}

/// 460002 always puts the rider on Skyriding and 460003 on Steady.
#[test]
fn the_directed_switches_set_their_style() {
    let Some(mut h) = switcher() else {
        return;
    };
    switch(&mut h, SWITCH_TO_STEADY);
    assert!(has(&h, FLIGHT_STYLE_STEADY) && !has(&h, FLIGHT_STYLE_SKYRIDING));
    assert_eq!(saved_style(&h), Some(FLIGHT_STYLE_STEADY));
    switch(&mut h, SWITCH_TO_SKYRIDING);
    switch(&mut h, SWITCH_TO_SKYRIDING);
    assert!(has(&h, FLIGHT_STYLE_SKYRIDING) && !has(&h, FLIGHT_STYLE_STEADY));
    assert_eq!(saved_style(&h), Some(FLIGHT_STYLE_SKYRIDING));
}

/// A character saved on Skyriding enters the world on Flight Style: Skyriding instead of
/// Steady.
#[test]
fn entering_the_world_restores_the_saved_flight_style() {
    let Some(mut h) = rider(225, true, &[]) else {
        return;
    };
    let db = crate::persistence::temp_database();
    crate::persistence_progression::save_flight_style(&db, CHARACTER, FLIGHT_STYLE_SKYRIDING);
    h.app
        .insert_resource(crate::persistence::GameDatabase { db });
    let player = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 5.0);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::persistence::ClientId(CHARACTER));
    h.advance(2);
    let world = h.app.world();
    assert!(has_aura(world, player, FLIGHT_STYLE_SKYRIDING));
    assert!(!has_aura(world, player, FLIGHT_STYLE_STEADY));
}
