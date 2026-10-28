//! Tigereye stacks are earned and consumed by real native casts.
use super::*;

fn stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        })
        .map_or(0, |aura| aura.stacks)
}

fn cast_when_ready(run: &mut Run, spell: u32, target: Entity) {
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                run.app.update();
                return;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress
                | crate::spell_cast::CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("spell {spell}: {reason:?}"),
        }
    }
    panic!("spell {spell} never became ready");
}

#[test]
#[ignore = "requires native 269 player profile"]
fn windwalker_tigereye_native_chi_cast_earns_zenith_crit() {
    let mut run = prepare_literal(269, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let enemy = run.targets[0];
    assert_eq!(
        stacks(&run, 1261724),
        10,
        "combat begins with native Tigereye stacks"
    );
    for spell in [100780, 100780, 113656] {
        cast_when_ready(&mut run, spell, enemy);
    }
    assert_eq!(stacks(&run, 1261724), 11, "three base Chi earn one stack");
    cast_when_ready(&mut run, 1249625, actor);
    assert_eq!(
        stacks(&run, 1261724),
        0,
        "Zenith consumes the earned stacks"
    );
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1249625 && aura.caster == actor)
        .unwrap();
    let crit = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(
        crit, 11.0,
        "Zenith grants one native percentage point per consumed stack"
    );
}
