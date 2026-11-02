//! Improved Adrenaline Rush fills the selected Outlaw follower's combo-point pool.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_improved_adrenaline_rush_fills_combo_points() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 395422));
    {
        let mut pools = run.app.world_mut().get_mut::<crate::spell_cast::powers::PowerPools>(run.actor).unwrap();
        pools.get_mut(PowerType::ComboPoints).unwrap().current = 0.0;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(13750, run.actor, None),
    ).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let (current, maximum) = crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::ComboPoints).unwrap();
    assert_eq!(current, maximum, "the real Adrenaline Rush cast fills combo points");
}
