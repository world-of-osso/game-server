//! Gift of the Wild enhances Mark of the Wild on its selected owner only.
use super::*;
use crate::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::stats::CharacterStats;

#[test]
#[ignore = "requires Guardian Gift of the Wild profile and native world data"]
fn gift_of_wild_doubles_only_owners_mark_and_reverts_on_removal() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let other = run.targets[0];
    assert!(auras::has_aura(run.app.world(), actor, 1262034));
    let before = run
        .app
        .world()
        .get::<CharacterStats>(actor)
        .unwrap()
        .versatility_pct;
    for target in [actor, other] {
        auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: actor,
                target,
                spell_id: 1126,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    for _ in 0..3 {
        run.app.update();
    }
    let value = |run: &Run, unit| {
        run.app
            .world()
            .get::<ActiveAuras>(unit)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1126)
            .unwrap()
            .effects
            .iter()
            .find(|e| e.index == 0)
            .unwrap()
            .amount
    };
    assert_eq!(value(&run, actor), 6.0);
    assert_eq!(value(&run, other), 3.0);
    let with_mark = run
        .app
        .world()
        .get::<CharacterStats>(actor)
        .unwrap()
        .versatility_pct;
    assert!((with_mark - before - 6.0).abs() < 0.01);
    auras::remove_auras(run.app.world_mut(), actor, RemoveReason::Cancelled, |a| {
        a.spell_id == 1262034
    });
    for _ in 0..3 {
        run.app.update();
    }
    assert_eq!(value(&run, actor), 3.0);
    let without_talent = run
        .app
        .world()
        .get::<CharacterStats>(actor)
        .unwrap()
        .versatility_pct;
    assert!((with_mark - without_talent - 3.0).abs() < 0.01);
}
