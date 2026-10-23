use super::*;
use crate::spell_cast::test_harness::Harness;
use crate::spell_info::SpellStore;

const TIGER_PALM: u32 = 100780;
const BLACKOUT_KICK: u32 = 100784;
const HIT_COMBO: u32 = 196740;
const HIT_COMBO_BUFF: u32 = 196741;
const PROVOKE: u32 = 115546;
const ROLL: u32 = 109132;

fn bindings() -> Bindings {
    Bindings {
        actions: [
            ("tiger_palm", TIGER_PALM),
            ("provoke", PROVOKE),
            ("roll", ROLL),
        ]
        .map(|(name, id)| (name.into(), id))
        .into(),
        buffs: [("hit_combo".into(), HIT_COMBO_BUFF)].into(),
        talents: [("hit_combo".into(), HIT_COMBO)].into(),
        resources: [
            ("energy".into(), PowerType::Energy),
            ("chi".into(), PowerType::Chi),
        ]
        .into(),
        global_cooldown_spell: TIGER_PALM,
    }
}

fn fixture(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::history_now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

fn monk() -> (Harness, Entity) {
    crate::followers::kit_tests::at_level(227734, 35, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    })
}

fn cast(h: &mut Harness, actor: Entity, spell: u32, target: Entity) {
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(h.app.world_mut(), actor, spell, Some(target), false)
        {
            Ok(()) => {
                h.advance(3);
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => h.advance(1),
            Err(reason) => panic!("native spell{spell} failed: {reason:?}"),
        }
    }
    panic!("native spell{spell} never became ready");
}

fn action(name: &str) -> Action {
    Action {
        name: name.into(),
        options: BTreeMap::new(),
    }
}

#[test]
#[ignore = "shared native ActionFacts RED: earned native buff/resource/talent bindings"]
fn native_apl_facts_follow_earned_buff_stacks_resources_and_repeat_removal() {
    let (mut h, actor) = monk();
    let b = bindings();
    let clock = fixture(h.app.world());
    let target = h.npc;
    cast(&mut h, actor, TIGER_PALM, target);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value("buff.hit_combo.up").unwrap(), 1.0);
    assert_eq!(f.value("buff.hit_combo.stack").unwrap(), 1.0);
    assert!(f.value("buff.hit_combo.remains").unwrap() > 0.0);
    assert_eq!(f.value("talent.hit_combo").unwrap(), 1.0);
    let (chi, max_chi) =
        crate::spell_cast::powers::power_value(h.app.world(), actor, PowerType::Chi).unwrap();
    assert!(chi > 0.0);
    assert_eq!(f.value("chi").unwrap(), f64::from(chi));
    assert_eq!(f.value("chi.max").unwrap(), f64::from(max_chi));
    assert_eq!(f.value("chi.deficit").unwrap(), f64::from(max_chi - chi));
    cast(&mut h, actor, BLACKOUT_KICK, target);
    assert_eq!(
        NativeFacts::new(h.app.world(), actor, &b, clock, 1)
            .value("buff.hit_combo.stack")
            .unwrap(),
        2.0
    );
    cast(&mut h, actor, BLACKOUT_KICK, target);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value("buff.hit_combo.up").unwrap(), 0.0);
    assert_eq!(f.value("buff.hit_combo.stack").unwrap(), 0.0);
    assert_eq!(f.value("buff.hit_combo.remains").unwrap(), 0.0);
}

#[test]
#[ignore = "shared native ActionFacts RED: successful native cooldown and action-bound GCD"]
fn native_apl_facts_read_native_cooldown_and_bind_control_action_gcd() {
    let (mut h, actor) = monk();
    let b = bindings();
    let clock = fixture(h.app.world());
    let target = h.npc;
    cast(&mut h, actor, PROVOKE, target);
    let native = h.app.world().resource::<SpellStore>().get(PROVOKE).unwrap();
    let duration = f64::from(native.cooldowns.as_ref().unwrap().recovery_ms) / 1000.0;
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    let remains = f.value("cooldown.provoke.remains").unwrap();
    assert!(remains > 0.0 && remains <= duration);
    assert_eq!(f.value("cooldown.provoke.duration").unwrap(), duration);
    assert_eq!(f.value("cooldown.provoke.up").unwrap(), 0.0);
    assert_eq!(
        f.value_for(&action("call_action_list"), "gcd").unwrap(),
        0.0
    );
    assert!(f.value_for(&action("tiger_palm"), "gcd").unwrap() > 0.0);
    assert!(f.value("gcd").unwrap_err().contains("action"));
    h.advance((duration * 20.0).ceil() as usize + 2);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value("cooldown.provoke.remains").unwrap(), 0.0);
    assert_eq!(f.value("cooldown.provoke.up").unwrap(), 1.0);
}

#[test]
#[ignore = "shared native ActionFacts RED: real Roll charge consumption/recharge"]
fn native_apl_facts_read_real_roll_charge_count_and_recharge_clock() {
    let (mut h, actor) = monk();
    let b = bindings();
    let clock = fixture(h.app.world());
    let categories = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(ROLL)
        .unwrap()
        .categories
        .as_ref()
        .unwrap();
    let max = f64::from(categories.max_charges);
    assert!(max > 0.0);
    cast(&mut h, actor, ROLL, actor);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value("cooldown.roll.charges").unwrap(), max - 1.0);
    assert!(f.value("cooldown.roll.full_recharge_time").unwrap() > 0.0);
}

#[test]
#[ignore = "shared native ActionFacts RED: one fixture clock for fight and every target"]
fn native_apl_facts_fixture_clock_and_unavailable_dungeon_ttd_are_explicit() {
    let (mut h, actor) = monk();
    let b = bindings();
    let clock = fixture(h.app.world());
    h.advance(40);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 3);
    let elapsed = f.value("time").unwrap();
    assert!((elapsed - 2.0).abs() < 0.001);
    assert!((f.value("fight_remains").unwrap() + elapsed - 300.0).abs() < 0.001);
    assert_eq!(
        f.value("target.time_to_die").unwrap(),
        f.value("fight_remains").unwrap()
    );
    assert_eq!(f.value("active_enemies").unwrap(), 3.0);
    assert_eq!(f.value("fight_style.patchwerk").unwrap(), 1.0);
    assert_eq!(f.value("fight_style.dungeonslice").unwrap(), 0.0);
    let dungeon = NativeFacts::new(
        h.app.world(),
        actor,
        &b,
        Encounter::Dungeon { started_at: 0.0 },
        3,
    );
    assert!(
        dungeon
            .value("target.time_to_die")
            .unwrap_err()
            .contains("unavailable")
    );
    assert!(
        dungeon
            .value("fight_remains")
            .unwrap_err()
            .contains("unavailable")
    );
    assert!(
        f.value("buff.missing_alias.up")
            .unwrap_err()
            .contains("unknown")
    );
}

#[test]
#[ignore = "shared native ActionFacts RED: native Charge generates displayed Rage rather than raw tenths"]
fn native_apl_facts_convert_real_charge_rage_using_native_display_modifier() {
    use crate::spell_cast::test_harness::harness_with;
    use shared::components::Position;
    const CHARGE: u32 = 100;
    let mut h = harness_with(Vec::new(), &[CHARGE], 1, true).unwrap();
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 20.0;
    let actor = h.caster;
    let target = h.npc;
    cast(&mut h, actor, CHARGE, target);
    let b = Bindings {
        resources: [("rage".into(), PowerType::Rage)].into(),
        ..Default::default()
    };
    let f = NativeFacts::new(h.app.world(), actor, &b, fixture(h.app.world()), 1);
    let (raw, _) =
        crate::spell_cast::powers::power_value(h.app.world(), actor, PowerType::Rage).unwrap();
    assert!(
        raw >= 200.0,
        "ordinary native Charge must earn its real200 raw Rage"
    );
    assert_eq!(f.value("rage").unwrap(), f64::from(raw) / 10.0);
}
