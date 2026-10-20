//! TC a352b1fa Unit::_UpdateAutoRepeatSpell (Unit.cpp3008–3044): Auto Shot75
//! (Attr2 AUTO_REPEAT) fires again whenever the ranged attack timer, reset by each
//! shot (Spell.cpp4229–4238), is ready. Moving does not stop the idle instant
//! repeat (Spell.cpp7175–7197); a failed range check skips the shot without
//! cancelling it for75 (Unit.cpp3029–3037). Cancel cast clears it (SpellHandler.cpp
//! 247–257, Unit.cpp3251–3259).
use super::test_harness::*;
use bevy::time::TimeUpdateStrategy;
use lightyear::prelude::*;
use shared::components::Position;
use shared::protocol::{CombatChannel, StopSpellCast};

const AUTO_SHOT: u32 = 75;
/// CharacterLoadout158 (Hunter, Purpose0) item: InventoryType26, ItemDelay3000.
const BEASTSOUL_RIFLE: u32 = 73000;
/// SpellRange5 maximum.
const AUTO_SHOT_RANGE: f32 = 40.0;

fn hunter() -> Harness {
    let mut h = harness_with(Vec::new(), &[AUTO_SHOT], 3, true).unwrap();
    let path = crate::test_data::world_db_path();
    h.app
        .world_mut()
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .world_mut()
        .insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::stats::EquippedItems {
            items: vec![(
                shared::protocol::EquipmentSlot::MainHand,
                shared::item_data::ItemDefinitionSource::Retail,
                BEASTSOUL_RIFLE,
            )],
        });
    h.advance(3);
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(caster)
        .unwrap();
    assert_eq!(stats.main_hand.speed, 3.0, "native rifle delay3000");
    assert_eq!(stats.haste_pct, 0.0, "fixture cadence excludes haste");
    h
}

/// Frames per native ranged attack at the harness's manual frame step.
fn shot_interval_frames(h: &Harness) -> usize {
    let frame_ms = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(step) => step.as_millis() as usize,
        _ => panic!("native harness must use a known manual frame step"),
    };
    3000 / frame_ms
}

fn shot_count(h: &Harness) -> usize {
    h.inbox
        .go
        .iter()
        .filter(|go| go.caster == h.caster.to_bits() && go.spell_id == AUTO_SHOT)
        .count()
}

/// Advance `frames`, returning the frame index of each new Auto Shot.
fn shots_over(h: &mut Harness, frames: usize) -> Vec<usize> {
    let mut shots = Vec::new();
    for frame in 0..frames {
        let before = shot_count(h);
        h.advance(1);
        shots.extend((before..shot_count(h)).map(|_| frame));
    }
    shots
}

fn start_auto_shot(h: &mut Harness) {
    let npc = h.npc;
    h.cast(AUTO_SHOT, Some(npc));
    let first = shots_over(h, 3);
    assert_eq!(first.len(), 1, "initial Auto Shot");
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

fn move_npc(h: &mut Harness, x: f32) {
    let npc = h.npc;
    let at = *h.app.world().get::<Position>(npc).unwrap();
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(Position { x, ..at });
}

#[test]
#[ignore = "RED: Auto Shot75 resolves once; no autorepeat state"]
fn class_completeness_native_auto_shot_repeats_at_ranged_weapon_speed() {
    let mut h = hunter();
    let interval = shot_interval_frames(&h);
    start_auto_shot(&mut h);
    let shots = shots_over(&mut h, interval * 3 + 2);
    assert_eq!(shots.len(), 3, "three repeats in9s: {shots:?}");
    for pair in shots.windows(2) {
        assert_eq!(pair[1] - pair[0], interval, "ranged cadence {shots:?}");
    }
}

#[test]
#[ignore = "RED: Auto Shot75 resolves once; no autorepeat state"]
fn class_completeness_native_auto_shot_keeps_firing_while_moving() {
    let mut h = hunter();
    let interval = shot_interval_frames(&h);
    start_auto_shot(&mut h);
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::networking::Moving(true));
    let shots = shots_over(&mut h, interval * 2 + 2);
    assert!(
        h.app
            .world()
            .get::<crate::networking::Moving>(caster)
            .unwrap()
            .0
    );
    assert_eq!(
        shots.len(),
        2,
        "moving hunter keeps its idle repeat: {shots:?}"
    );
}

#[test]
#[ignore = "RED: Auto Shot75 resolves once; no autorepeat state"]
fn class_completeness_native_auto_shot_out_of_range_pauses_without_cancelling() {
    let mut h = hunter();
    let interval = shot_interval_frames(&h);
    start_auto_shot(&mut h);
    let near = h.app.world().get::<Position>(h.npc).unwrap().x;
    move_npc(&mut h, AUTO_SHOT_RANGE + 10.0);
    assert!(shots_over(&mut h, interval * 2).is_empty(), "out of range");
    move_npc(&mut h, near);
    // The timer stayed ready; the retained repeat fires without a new cast.
    let resumed = shots_over(&mut h, interval + 2);
    assert_eq!(resumed.len(), 1, "back in range: {resumed:?}");
    assert!(resumed[0] < 3, "ready timer fires at once: {resumed:?}");
}

#[test]
#[ignore = "RED: Auto Shot75 resolves once; no autorepeat state"]
fn class_completeness_native_auto_shot_stop_cast_cancels_the_repeat() {
    let mut h = hunter();
    let interval = shot_interval_frames(&h);
    start_auto_shot(&mut h);
    let repeat = shots_over(&mut h, interval + 2);
    assert_eq!(repeat.len(), 1, "one repeat before cancelling: {repeat:?}");
    let connection = h.connection;
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    assert!(
        shots_over(&mut h, interval * 3).is_empty(),
        "cancelled repeat"
    );
}
