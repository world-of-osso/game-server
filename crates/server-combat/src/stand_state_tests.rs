//! Player stand state on real 50 ms ticks against data/world.db:
//! requests, the stand-up rules and the auras that need the player seated.

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Health, PlayerMotion, PlayerStandState, StandState};
use shared::protocol::{CombatChannel, StandStateIntent};

use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::*;

const MAGE: u8 = 8;
/// Food, Tough Hunk of Bread 4540: OBS_MOD_HEALTH 3% of max health every second
/// for 20 s, Standing aura interrupt, `SPELL_ATTR0_ALLOW_WHILE_SITTING`.
const BREAD_FOOD: u32 = 1_291_822;
/// Food, Shiny Red Apple 4536: the same food spell shape as the bread's.
const APPLE_FOOD: u32 = 1_291_825;
/// Drink, Refreshing Spring Water 159: OBS_MOD_POWER 1% of max mana every second.
const WATER_DRINK: u32 = 1_291_950;
/// Frostbolt: a 1.5 s cast without `SPELL_ATTR0_ALLOW_WHILE_SITTING`.
const FROSTBOLT: u32 = 116;
/// 2 s of 50 ms frames.
const FRAMES_PER_TICK: usize = 40;

fn stand_state(h: &Harness) -> StandState {
    h.app.world().get::<PlayerStandState>(h.caster).unwrap().0
}

fn set_stand_state(h: &mut Harness, state: StandState) {
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(PlayerStandState(state));
}

/// `CMSG_STAND_STATE_CHANGE` from the caster's client; like a cast it arrives 3 frames
/// later.
fn request(h: &mut Harness, state: StandState) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<StandStateIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StandStateIntent { state });
    h.advance(3);
}

fn set_health(h: &mut Harness, current: f32, max: f32) {
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(Health { current, max });
}

fn set_combat(h: &mut Harness, in_combat: bool) {
    h.app
        .world_mut()
        .get_mut::<CombatStatus>(h.caster)
        .unwrap()
        .0 = in_combat;
}

fn auras(h: &Harness) -> Vec<u32> {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .map(|auras| auras.iter().map(|aura| aura.spell_id).collect())
        .unwrap_or_default()
}

/// Health gained over each of `ticks` 2 s regen ticks, measured from the frame after
/// the next tick so every window holds exactly one.
fn regen_gains(h: &mut Harness, ticks: usize) -> Vec<f32> {
    let mut gains = Vec::new();
    let mut last = h.health(h.caster);
    for _ in 0..ticks {
        h.advance(FRAMES_PER_TICK);
        let now = h.health(h.caster);
        gains.push(now - last);
        last = now;
    }
    gains
}

fn mage(ids: &[u32]) -> Option<Harness> {
    let mut h = harness(Vec::new(), ids, MAGE)?;
    h.advance(1);
    set_combat(&mut h, false);
    Some(h)
}

#[test]
fn a_player_may_ask_to_stand_sit_sleep_or_kneel_and_nothing_else() {
    let Some(mut h) = mage(&[]) else {
        return;
    };
    assert_eq!(stand_state(&h), StandState::Stand);
    request(&mut h, StandState::Sit);
    assert_eq!(stand_state(&h), StandState::Sit);
    // HandleStandStateChangeOpcode drops chair and other states.
    request(&mut h, StandState::SitLowChair);
    assert_eq!(stand_state(&h), StandState::Sit);
    request(&mut h, StandState::Dead);
    assert_eq!(stand_state(&h), StandState::Sit);
    request(&mut h, StandState::Kneel);
    assert_eq!(stand_state(&h), StandState::Kneel);
    request(&mut h, StandState::Sleep);
    assert_eq!(stand_state(&h), StandState::Sleep);
    request(&mut h, StandState::Stand);
    assert_eq!(stand_state(&h), StandState::Stand);
}

/// Player::RegenerateHealth (Player.cpp:1774-1775): ×1.5 while not standing. A level
/// 10 mage with 1000 max health regains 20 per tick standing, 30 sitting, asleep or
/// kneeling.
#[test]
fn a_seated_sleeping_or_kneeling_player_regains_half_again_as_much_health() {
    let Some(mut h) = mage(&[]) else {
        return;
    };
    set_health(&mut h, 100.0, 1000.0);
    h.advance(FRAMES_PER_TICK);
    assert_eq!(regen_gains(&mut h, 2), vec![20.0, 20.0]);
    for state in [StandState::Sit, StandState::Sleep, StandState::Kneel] {
        set_stand_state(&mut h, state);
        assert_eq!(regen_gains(&mut h, 2), vec![30.0, 30.0], "{state:?}");
    }
    set_stand_state(&mut h, StandState::Stand);
    assert_eq!(regen_gains(&mut h, 1), vec![20.0]);
}

/// Unit::_ApplyAura (Unit.cpp:3584-3586) sits a standing player down for an aura with
/// the Standing interrupt flag; standing up (SetStandState, Unit.cpp:10806-10807)
/// removes it. Food is eaten out of combat (`SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL`);
/// entering combat after does not end it (no EnteringCombat interrupt), and in combat the
/// player regenerates nothing, so every gain is the food's 3% of 1000 health per second.
#[test]
fn bread_sits_the_player_down_heals_while_seated_and_standing_up_ends_it() {
    let Some(mut h) = mage(&[BREAD_FOOD]) else {
        return;
    };
    set_health(&mut h, 100.0, 1000.0);
    let caster = h.caster;
    crate::spell_cast::cast_granted(h.app.world_mut(), caster, BREAD_FOOD, None).unwrap();
    set_combat(&mut h, true);
    assert_eq!(stand_state(&h), StandState::Sit);
    assert_eq!(auras(&h), vec![BREAD_FOOD]);
    h.advance(FRAMES_PER_TICK);
    // Two 1 s food ticks: 60 health.
    assert_eq!(h.health(caster), 160.0);
    request(&mut h, StandState::Stand);
    assert!(auras(&h).is_empty(), "{:?}", auras(&h));
    let stood = h.health(caster);
    h.advance(FRAMES_PER_TICK);
    assert_eq!(h.health(caster), stood);
}

/// Eating while kneeling keeps the player kneeling (only a standing player sits).
#[test]
fn food_keeps_a_kneeling_player_kneeling() {
    let Some(mut h) = mage(&[BREAD_FOOD]) else {
        return;
    };
    set_stand_state(&mut h, StandState::Kneel);
    let caster = h.caster;
    crate::spell_cast::cast_granted(h.app.world_mut(), caster, BREAD_FOOD, None).unwrap();
    assert_eq!(stand_state(&h), StandState::Kneel);
    assert_eq!(auras(&h), vec![BREAD_FOOD]);
}

/// SpellInfo::IsAuraExclusiveBySpecificWith: a new food replaces the old one, a drink
/// stays beside it.
#[test]
fn a_new_food_replaces_the_old_one_and_a_drink_keeps_it() {
    let Some(mut h) = mage(&[BREAD_FOOD, APPLE_FOOD, WATER_DRINK]) else {
        return;
    };
    let caster = h.caster;
    let world = h.app.world_mut();
    crate::spell_cast::cast_granted(world, caster, BREAD_FOOD, None).unwrap();
    crate::spell_cast::cast_granted(world, caster, WATER_DRINK, None).unwrap();
    crate::spell_cast::cast_granted(world, caster, APPLE_FOOD, None).unwrap();
    let mut active = auras(&h);
    active.sort_unstable();
    assert_eq!(active, vec![APPLE_FOOD, WATER_DRINK]);
}

/// HandleMovementOpcode (MovementHandler.cpp:589-590): moving stands a seated player
/// up, which ends its food.
#[test]
fn moving_stands_the_player_up_and_ends_its_food() {
    let Some(mut h) = mage(&[BREAD_FOOD]) else {
        return;
    };
    let caster = h.caster;
    crate::spell_cast::cast_granted(h.app.world_mut(), caster, BREAD_FOOD, None).unwrap();
    assert_eq!(stand_state(&h), StandState::Sit);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(PlayerMotion(PlayerMotion::FORWARD));
    h.advance(2);
    assert_eq!(stand_state(&h), StandState::Stand);
    assert!(auras(&h).is_empty());
}

/// A spell without SPELL_ATTR0_ALLOW_WHILE_SITTING stands its caster up; food (which
/// has it) is eaten seated.
#[test]
fn casting_frostbolt_stands_a_sleeping_player_up_but_eating_does_not() {
    let Some(mut h) = mage(&[FROSTBOLT, BREAD_FOOD]) else {
        return;
    };
    set_stand_state(&mut h, StandState::Sit);
    let caster = h.caster;
    crate::spell_cast::cast_granted(h.app.world_mut(), caster, BREAD_FOOD, None).unwrap();
    assert_eq!(stand_state(&h), StandState::Sit);
    set_stand_state(&mut h, StandState::Sleep);
    let npc = h.npc;
    h.cast_and_deliver(FROSTBOLT, Some(npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(stand_state(&h), StandState::Stand);
}

/// Unit::DealDamage (Unit.cpp:1185-1186): a hit stands the player up, a periodic tick
/// does not.
#[test]
fn a_hit_stands_a_kneeling_player_up_but_a_periodic_tick_does_not() {
    /// Poison (a creature spell): PERIODIC_DAMAGE 5 every 3 s.
    const POISON: u32 = 13_298;
    let Some(mut h) = mage(&[POISON]) else {
        return;
    };
    let (caster, npc) = (h.caster, h.npc);
    set_stand_state(&mut h, StandState::Kneel);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: npc,
            target: caster,
            spell_id: POISON,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let before = h.health(caster);
    h.advance(62);
    assert!(h.health(caster) < before, "Poison ticked");
    assert_eq!(stand_state(&h), StandState::Kneel);
    crate::combat::deal_direct_damage(h.app.world_mut(), npc, caster, 10.0);
    assert_eq!(stand_state(&h), StandState::Stand);
}
