//! Native state regressions for the shared Dev/Havoc fact assignment.
use super::*;

#[test]
#[ignore = "requires native DB inputs: selected ranks, not learned-spell membership"]
fn native_apl_extensions_talent_rank_reads_real_multirank_allocation() {
    use crate::class_progression::TraitLoadout;
    use crate::spell_info::ClassSpells;
    let (h, actor) = crate::followers::kit_tests::at_level(227734, 55, |_| {});
    let world = h.app.world();
    let profile = &world
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .profile;
    let class = world
        .resource::<ClassSpells>()
        .classes
        .iter()
        .find(|class| class.class_id == profile.class_id)
        .unwrap();
    let traits = world.get::<TraitLoadout>(actor).unwrap();
    let selected = traits
        .source_config()
        .0
        .iter()
        .find(|entry| entry.total() > 1)
        .expect("real native55 starter selects a multi-rank talent");
    let spell = class
        .traits
        .node(selected.node_id)
        .unwrap()
        .entry(selected.entry_id)
        .unwrap()
        .spell_id;
    assert!(spell > 0, "selected native definition has a spell");
    let b = Bindings {
        talents: [
            ("selected_native".into(), spell),
            ("learned_utility".into(), PROVOKE),
        ]
        .into(),
        ..Default::default()
    };
    let f = NativeFacts::new(world, actor, &b, fixture(world), 1);
    assert_eq!(
        f.value("talent.selected_native.rank").unwrap(),
        f64::from(selected.total())
    );
    assert_eq!(f.value("talent.learned_utility.rank").unwrap(), 0.0);
}

#[test]
#[ignore = "requires native DB inputs: earned Hit Combo cap and loss"]
fn native_apl_extensions_at_max_stacks_tracks_earned_cap_and_removal() {
    let (mut h, actor) = monk();
    let b = bindings();
    let clock = fixture(h.app.world());
    let read = |h: &Harness| {
        NativeFacts::new(h.app.world(), actor, &b, clock, 1)
            .value("buff.hit_combo.at_max_stacks")
            .unwrap()
    };
    assert_eq!(read(&h), 0.0);
    let target = h.npc;
    cast(&mut h, actor, TIGER_PALM, target);
    let cap = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == HIT_COMBO_BUFF)
        .unwrap()
        .max_stacks;
    assert!(cap > 1, "native Hit Combo must genuinely stack");
    assert_eq!(read(&h), 0.0);
    for index in 1..cap {
        let spell = if index % 2 == 1 {
            BLACKOUT_KICK
        } else {
            TIGER_PALM
        };
        cast(&mut h, actor, spell, target);
    }
    assert_eq!(read(&h), 1.0);
    let repeated = if cap % 2 == 0 {
        BLACKOUT_KICK
    } else {
        TIGER_PALM
    };
    cast(&mut h, actor, repeated, target);
    assert_eq!(read(&h), 0.0, "native repeat loses Hit Combo");
}

#[test]
#[ignore = "requires native DB inputs: spell_haste is a duration factor"]
fn native_apl_extensions_spell_haste_reads_real_derived_martial_agility() {
    let (mut h, actor) = crate::followers::kit_tests::at_level(227734, 55, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.advance(20);
    assert!(crate::auras::has_aura(h.app.world(), actor, 1272685));
    let haste = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .haste_pct;
    assert!(haste > 0.0, "native Martial Agility supplies actual haste");
    let b = bindings();
    let f = NativeFacts::new(h.app.world(), actor, &b, fixture(h.app.world()), 1);
    let expected = 1.0 / (1.0 + f64::from(haste) / 100.0);
    assert!((f.value("spell_haste").unwrap() - expected).abs() < 0.000001);
}

#[test]
#[ignore = "requires native DB inputs: contextual charges and full recharge"]
fn native_apl_extensions_contextual_charges_follow_actual_roll_consumption() {
    let (mut h, actor) = monk();
    let b = bindings();
    let clock = fixture(h.app.world());
    let roll = action("roll");
    let max = f64::from(
        h.app
            .world()
            .resource::<SpellStore>()
            .get(ROLL)
            .unwrap()
            .categories
            .as_ref()
            .unwrap()
            .max_charges,
    );
    assert!(max > 1.0, "native Roll genuinely has multiple charges");
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value_for(&roll, "charges").unwrap(), max);
    assert_eq!(f.value_for(&roll, "charges_fractional").unwrap(), max);
    assert_eq!(f.value_for(&roll, "full_recharge_time").unwrap(), 0.0);
    cast(&mut h, actor, ROLL, actor);
    let f = NativeFacts::new(h.app.world(), actor, &b, clock, 1);
    assert_eq!(f.value_for(&roll, "charges").unwrap(), max - 1.0);
    assert_eq!(
        f.value_for(&roll, "charges").unwrap(),
        f.value("cooldown.roll.charges").unwrap()
    );
    assert_eq!(
        f.value_for(&roll, "full_recharge_time").unwrap(),
        f.value("cooldown.roll.full_recharge_time").unwrap()
    );
    let fractional = f.value_for(&roll, "charges_fractional").unwrap();
    assert!(fractional >= max - 1.0 && fractional < max);
    assert!(f.value("charges").unwrap_err().contains("action"));
}
