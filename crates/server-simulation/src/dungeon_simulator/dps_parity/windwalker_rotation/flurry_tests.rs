//! Native Flurry earning/release boundaries; no aura, stack, power or stat grants.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::WeaponType;
use shared::protocol::{CombatLogEvent, CombatLogKind};

type Run = super::super::Run;
const PASSIVE: u32 = 450615;
const CHARGE: u32 = 451021;
const STRIKE: u32 = 450617;
const AREA: u32 = 451250;
const DUAL_THREAT: u32 = 451839;
const FOF: u32 = 113656;
const ONE_VERSUS_MANY: u32 = 450988;

fn prepare(selected: bool) -> Run {
    let (profile, mut traits) = super::super::read_reference(269).unwrap();
    if !selected {
        // Flurry is a granted Shado-Pan root. Unselect the hero subtree itself,
        // preserving the frozen profile's class/spec purchases before progression.
        let path = super::super::inputs::directory()
            .unwrap()
            .join("269.selected-traits-active.json");
        let exported: Vec<Value> = super::super::inputs::read_json(&path).unwrap();
        let retained: std::collections::BTreeSet<_> = exported
            .iter()
            .filter(|entry| entry["tree"] == "class" || entry["tree"] == "spec")
            .map(|entry| entry["entry_id"].as_u64().unwrap() as u32)
            .collect();
        let count = traits.len();
        traits.retain(|entry| retained.contains(&entry.entry_id));
        assert!(traits.len() < count);
    }
    // This fixture qualifies ordinary earned-charge release, not the independent
    // Wisdom/Stand Ready batches. Declare those two native talent inputs unselected;
    // hero_flurry_tests keeps the full frozen selection and qualifies their producers.
    traits.retain(|entry| !matches!(entry.entry_id, 125073 | 135956));
    let identity = super::super::inputs::read_actor(&profile).unwrap();
    let mut run = super::super::prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    run.spec_driver = Some(super::prepare_driver(&mut run).unwrap());
    let applied = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap()
        .active_talent_spells()
        .any(|spell| spell.spell_id == PASSIVE);
    assert_eq!(applied, selected, "native declared hero-selection input");
    assert_eq!(
        crate::auras::has_aura(run.app.world(), run.actor, PASSIVE),
        selected,
        "native progression must apply only the selected hero passive"
    );
    let started_at = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started_at);
    run
}

fn events(run: &Run) -> Vec<CombatLogEvent> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.source == Some(run.actor.to_bits()))
        .cloned()
        .collect()
}

fn deliveries(run: &Run, spell: u32) -> Vec<CombatLogEvent> {
    events(run)
        .into_iter()
        .filter(|event| {
            event.spell_id == Some(spell)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .collect()
}

fn charges(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == CHARGE && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn cap(run: &Run) -> u8 {
    let store = run.app.world().resource::<crate::spell_info::SpellStore>();
    let cap = store
        .get(CHARGE)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura;
    assert_eq!(cap, 30, "CURRENT native451021 cap");
    cap as u8
}

fn earned_per_hit(run: &Run, critical: bool) -> u8 {
    let world = run.app.world();
    let stats = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    let index = match stats.main_hand.kind {
        WeaponType::TwoHand => 1,
        WeaponType::OneHand | WeaponType::Dagger => 0,
        other => panic!("native melee weapon prerequisite: {other:?}"),
    };
    assert!(
        stats.off_hand.is_none(),
        "this frozen reference uses one native2H weapon"
    );
    let auras = world.get::<ActiveAuras>(run.actor).unwrap();
    let passive = auras.iter().find(|aura| aura.spell_id == PASSIVE).unwrap();
    let base = passive
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .amount;
    let crit_bonus = auras
        .iter()
        .find(|aura| aura.spell_id == ONE_VERSUS_MANY)
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    (base * if critical { 1.0 + crit_bonus } else { 1.0 }) as u8
}

#[test]
#[ignore = "actual selected Dual Threat replaces native white impacts without consecutive procs"]
fn literal_ww_native_dual_threat_replaces_eligible_white_impacts() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(true);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == 451823)
    );
    assert!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .off_hand
            .is_none()
    );
    start_white(&mut run);
    for _ in 0..1200 {
        let before = events(&run).len();
        run.app.update();
        let observed = events(&run);
        let impacts = observed[before..]
            .iter()
            .filter(|event| {
                (event.spell_id.is_none() || event.spell_id == Some(DUAL_THREAT))
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .count();
        assert!(
            impacts <= 1,
            "actual single-weapon attempt must replace white damage, not add a second impact"
        );
        let swings = observed
            .into_iter()
            .filter(|event| {
                (event.spell_id.is_none() || event.spell_id == Some(DUAL_THREAT))
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .collect::<Vec<_>>();
        if swings.len() < 20 {
            continue;
        }
        assert!(
            swings.first().unwrap().spell_id.is_none(),
            "first impact must remain white"
        );
        assert!(
            swings
                .iter()
                .any(|event| event.spell_id == Some(DUAL_THREAT)),
            "selected native Dual Threat must actually replace a white impact"
        );
        assert!(
            swings
                .iter()
                .filter(|event| event.spell_id == Some(DUAL_THREAT))
                .all(|event| event.kind == CombatLogKind::Damage && event.amount > 0),
            "actual Dual Threat child cannot roll white/spell avoidance against the native dummy"
        );
        assert!(
            swings
                .windows(2)
                .all(|pair| pair[0].spell_id != Some(DUAL_THREAT)
                    || pair[1].spell_id != Some(DUAL_THREAT)),
            "replacement gate must forbid consecutive Dual Threat impacts"
        );
        return;
    }
    panic!("actual native swing ordering never reached twenty attempts");
}

fn start_white(run: &mut Run) {
    let world = run.app.world_mut();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    crate::combat::start_attack(&mut commands, run.actor, run.targets[0], None);
    queue.apply(world);
}

#[test]
#[ignore = "native selected white impacts earn source Flurry charges, crit multiplier and cap"]
fn literal_ww_native_flurry_white_earns_native_charges_and_caps() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(true);
    start_white(&mut run);
    let mut expected = 0;
    let mut critical = false;
    for _ in 0..1200 {
        let before = events(&run).len();
        run.app.update();
        for event in &events(&run)[before..] {
            if (event.spell_id.is_none() || event.spell_id == Some(DUAL_THREAT))
                && event.kind == CombatLogKind::Damage
            {
                critical |= event.crit;
                expected = (expected + earned_per_hit(&run, event.crit)).min(cap(&run));
                assert_eq!(
                    charges(&run),
                    expected,
                    "real white or replacing Dual Threat impact must earn native451021 once"
                );
            }
        }
        if expected == cap(&run) && critical {
            return;
        }
    }
    panic!(
        "native white/crit/cap prerequisites not observed: expected={expected}, critical={critical}"
    );
}

#[test]
#[ignore = "actual ordinary native Dual Threat damage earns source Flurry charges, no producer claim"]
fn literal_ww_native_flurry_dual_threat_impact_earns_charges() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(true);
    let mut expected = 0;
    for _ in 0..12 {
        let before = deliveries(&run, DUAL_THREAT).len();
        super::tests::cast(&mut run, DUAL_THREAT);
        for event in &deliveries(&run, DUAL_THREAT)[before..] {
            if event.kind == CombatLogKind::Damage {
                expected = (expected + earned_per_hit(&run, event.crit)).min(cap(&run));
                assert_eq!(
                    charges(&run),
                    expected,
                    "actual native451839 impact must earn451021"
                );
                return;
            }
        }
    }
    panic!("native451839 never landed");
}

#[test]
#[ignore = "native literal white earnings must release both native children on actual FoF execute"]
fn literal_ww_native_flurry_fof_releases_earned_charges_and_native_children() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(true);
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>)
        .downcast::<Rotation>()
        .unwrap();
    let mut earned_before_cast = None;
    for _ in 0..6000 {
        let before = charges(&run);
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        if events(&run)
            .iter()
            .any(|event| event.spell_id == Some(FOF) && event.kind == CombatLogKind::CastSuccess)
        {
            earned_before_cast = Some(before);
            break;
        }
        run.app.update();
    }
    let earned = earned_before_cast.expect("actual literal native FoF cast prerequisite");
    assert!(
        events(&run)
            .iter()
            .any(|event| event.spell_id.is_none() && event.kind == CombatLogKind::Damage),
        "native white earning prerequisite"
    );
    assert_eq!(
        charges(&run),
        0,
        "actual FoF execute consumes the live native charge stack once"
    );
    let cast_at = crate::spell_cast::now(run.app.world());
    let frame_ms = run.app.world().resource::<Time>().delta().as_millis() as u32;
    let mut delivered_at = Vec::new();
    for _ in 0..100 {
        run.app.update();
        let count = deliveries(&run, STRIKE).len();
        let elapsed = ((crate::spell_cast::now(run.app.world()) - cast_at) * 1000.0).round() as u32;
        delivered_at.resize(count, elapsed);
    }
    let strike = deliveries(&run, STRIKE);
    let area = deliveries(&run, AREA);
    assert!(
        !strike.is_empty() && !area.is_empty(),
        "earned native FoF must release450617 and451250, not only cast its root"
    );
    assert!(
        earned > 0,
        "charges must have been naturally earned before FoF"
    );
    assert_eq!(strike.len(), usize::from(earned));
    assert_eq!(area.len(), usize::from(earned));
    for (index, actual) in delivered_at.into_iter().enumerate() {
        let nominal = index as u32 * 150;
        assert!(
            actual >= nominal && actual <= nominal + frame_ms + 1,
            "SimC150ms spacing, quantized only by the actual native frame: child{index} at{actual}ms"
        );
    }
    assert!(
        strike
            .iter()
            .chain(&area)
            .any(|event| event.kind == CombatLogKind::Damage && event.amount > 0)
    );
}

#[test]
#[ignore = "a real avoided white impact before cap must not earn Flurry charges"]
fn literal_ww_native_flurry_missed_white_does_not_earn_charges() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(true);
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>)
        .downcast::<Rotation>()
        .unwrap();
    for _ in 0..6000 {
        let before = charges(&run);
        let count = events(&run).len();
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        run.app.update();
        let new = events(&run)[count..].to_vec();
        let missed = new
            .iter()
            .any(|event| event.spell_id.is_none() && matches!(event.kind, CombatLogKind::Miss(_)));
        let landed = new
            .iter()
            .any(|event| event.spell_id.is_none() && event.kind == CombatLogKind::Damage);
        let released = new
            .iter()
            .any(|event| event.spell_id == Some(FOF) && event.kind == CombatLogKind::CastSuccess);
        if missed && !landed && !released && before < cap(&run) {
            assert_eq!(
                charges(&run),
                before,
                "actual avoided white cannot earn a charge"
            );
            return;
        }
    }
    panic!("actual avoided white before source cap prerequisite not observed");
}

#[test]
#[ignore = "unselected native owner has real white impacts but no charge or Flurry children"]
fn literal_ww_native_flurry_unselected_owner_does_not_earn_charges() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(false);
    start_white(&mut run);
    for _ in 0..120 {
        run.app.update();
    }
    assert!(
        events(&run)
            .iter()
            .any(|event| event.spell_id.is_none() && event.kind == CombatLogKind::Damage)
    );
    assert_eq!(charges(&run), 0);
    assert!(deliveries(&run, STRIKE).is_empty() && deliveries(&run, AREA).is_empty());
}

#[test]
#[ignore = "selected owner with no earned white/DT charge casts FoF through actual Tiger Palm resources"]
fn literal_ww_native_flurry_no_charge_fof_does_not_invent_children() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(true);
    super::tests::cast(&mut run, 100780);
    super::tests::cast(&mut run, 100780);
    super::tests::cast(&mut run, FOF);
    for _ in 0..100 {
        run.app.update();
    }
    assert!(
        events(&run)
            .iter()
            .any(|event| event.spell_id == Some(FOF) && event.kind == CombatLogKind::CastSuccess)
    );
    assert!(
        !events(&run).iter().any(|event| event.spell_id.is_none()),
        "no actual white earning boundary in this control"
    );
    assert!(deliveries(&run, DUAL_THREAT).is_empty());
    assert_eq!(charges(&run), 0);
    assert!(deliveries(&run, STRIKE).is_empty() && deliveries(&run, AREA).is_empty());
}
