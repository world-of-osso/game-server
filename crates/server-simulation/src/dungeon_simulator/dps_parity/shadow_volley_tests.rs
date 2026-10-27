//! SimC76f8118 sc_priest_shadow.cpp1025–1079/1365–1399/1829–1846/1960–2000.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const VOIDFORM: u32 = 228260;
const VOLLEY: u32 = 1242173;
const BOLT: u32 = 1242189;
const VOLLEY_USES: u32 = 1242171;
const CRUSHING_VOID: u32 = 1279437;

fn stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        .map_or(0, |aura| aura.stacks)
}

fn native_value(run: &Run, spell: u32, index: u8) -> usize {
    run.app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .base_points as usize
}

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn cast(run: &mut Run, spell: u32) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], None),
    )
    .unwrap_or_else(|error| panic!("ordinary spell{spell}: {error}"));
    for _ in 0..100 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
        {
            return;
        }
        run.app.update();
    }
    panic!("native spell{spell} cast did not finish");
}

fn bolt_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(BOLT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn consume_voidform_uses(run: &mut Run) -> usize {
    let uses = native_value(run, VOIDFORM, 2);
    assert_eq!(usize::from(stacks(run, VOLLEY_USES)), uses);
    for remaining in (0..uses).rev() {
        cast(run, VOLLEY);
        advance(run, 60);
        assert_eq!(usize::from(stacks(run, VOLLEY_USES)), remaining);
    }
    uses
}

fn ready(run: &mut Run, name: &str, spell: u32) -> bool {
    use literal_rotation::equipment::Equipment;
    use literal_rotation::execution::{NativeAction, Prepared};
    let equipment = Equipment::load(run.app.world(), run.actor, &mut Default::default()).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [NativeAction {
            name,
            spell,
            talent: None,
            self_target: false,
        }],
        false,
    )
    .unwrap();
    let action = server::simc_apl::Action {
        source_row: None,
        name: name.into(),
        target: None,
        options: Default::default(),
    };
    prepared.execution(&action, &equipment).unwrap().is_some()
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_volley_voidform_funds_native_uses_and_ordinary_casts_consume_them() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    cast(&mut run, VOIDFORM);
    assert_eq!(
        usize::from(stacks(&run, VOLLEY_USES)),
        native_value(&run, VOIDFORM, 2)
    );
    advance(&mut run, 5);
    let early = bolt_hits(&run);
    assert!(
        early > 0 && early < native_value(&run, VOLLEY, 0),
        "free Volley bolts must be staggered50ms, observed {early} by250ms"
    );
    advance(&mut run, 55);
    let uses = consume_voidform_uses(&mut run);
    assert_eq!(bolt_hits(&run), (1 + uses) * native_value(&run, VOLLEY, 0));
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(VOLLEY)
        .unwrap()
        .clone();
    assert!(
        crate::spell_cast::read_native_cast_start(
            run.app.world_mut(),
            run.actor,
            &spell,
            run.targets[0]
        )
        .is_err(),
        "ordinary Volley is unavailable after the last earned use"
    );
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_volley_native_four_piece_slam_earns_an_actual_outside_voidform_cast() {
    let mut run = prepare_literal(258, 1).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1296580),
        "native four-piece equipped"
    );
    assert_eq!(
        native_value(&run, 1296580, 0),
        100,
        "native deterministic chance"
    );
    engage_fixture_opponents(&mut run);
    cast(&mut run, 1227280);
    advance(&mut run, 60);
    assert_eq!(stacks(&run, CRUSHING_VOID), 1);
    assert!(
        !ready(&mut run, "voidform", VOIDFORM),
        "literal Voidform cannot alias into a Volley"
    );
    assert!(ready(&mut run, "void_volley", VOLLEY));
    cast(&mut run, VOLLEY);
    advance(&mut run, 60);
    assert_eq!(stacks(&run, CRUSHING_VOID), 0);
    assert_eq!(bolt_hits(&run), native_value(&run, VOLLEY, 0));
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_volley_natural_voidform_expiry_earns_one_crushing_void_use() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    cast(&mut run, VOIDFORM);
    advance(&mut run, 60);
    consume_voidform_uses(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 194249));
    assert_eq!(stacks(&run, CRUSHING_VOID), 0);
    advance(&mut run, 500);
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 194249));
    assert_eq!(stacks(&run, CRUSHING_VOID), 1);
    cast(&mut run, VOLLEY);
    advance(&mut run, 60);
    assert_eq!(stacks(&run, CRUSHING_VOID), 0);
}
