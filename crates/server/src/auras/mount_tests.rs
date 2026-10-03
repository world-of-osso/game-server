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

fn rider(riding: u16, flight_style: bool) -> Option<Harness> {
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
fn mount(h: &mut Harness) {
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

#[test]
fn players_entering_the_world_carry_flight_style_steady() {
    let Some(h) = rider(225, true) else {
        return;
    };
    assert!(has(&h, FLIGHT_STYLE_STEADY));
}

#[test]
fn expert_riding_mounts_the_golden_gryphon_with_standard_steady_flight() {
    let Some(mut h) = rider(225, true) else {
        return;
    };
    mount(&mut h);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app.world().get::<Mounted>(h.caster),
        Some(&Mounted {
            mount_display_id: GOLDEN_GRYPHON_DISPLAY
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
        let Some(mut h) = rider(riding, true) else {
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
    let Some(mut h) = rider(225, false) else {
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
    let Some(mut h) = rider(225, true) else {
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
    let Some(mut h) = rider(225, true) else {
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
    let Some(mut h) = rider(225, true) else {
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
