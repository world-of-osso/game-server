//! Retail-selected Red Thirst shortens Vampiric Blood by the Runic Power actually spent.
use super::*;
use shared::components::PowerType;

const VAMPIRIC_BLOOD: u32 = 55233;
const DEATH_STRIKE: u32 = 49998;

fn remaining_ms(run: &Run, id: u32) -> u64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
}

#[test]
#[ignore = "requires native Blood profile and world database"]
fn red_thirst_shortens_vampiric_blood_per_runic_power_spent() {
    let _seed = crate::random::seed_scope(70448);
    let (profile, _) = read_reference(250).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let traits = super::blood_umbilicus_tests::retail_traits();
    assert!(
        traits.iter().any(|entry| entry.entry_id == 96263),
        "retail Red Thirst"
    );
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let enemy = run.targets[0];
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        VAMPIRIC_BLOOD,
        Some(actor),
        false,
    )
    .unwrap();
    let before = remaining_ms(&run, VAMPIRIC_BLOOD);
    assert!(before > 60_000, "Vampiric Blood cooldown running: {before}");
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        PowerType::RunicPower,
        1000.0,
    );
    let power = |run: &Run| {
        run.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap()
            .get(PowerType::RunicPower)
            .unwrap()
            .current
    };
    let power_before = power(&run);
    let started = crate::spell_cast::now(run.app.world());
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, DEATH_STRIKE, Some(enemy), false)
        .unwrap();
    let paid = (power_before - power(&run)) / 10.0;
    assert!(paid > 0.0, "Death Strike paid Runic Power");
    let elapsed_ms = ((crate::spell_cast::now(run.app.world()) - started) * 1000.0) as u64;
    let after = remaining_ms(&run, VAMPIRIC_BLOOD);
    // DB2 205723: 2000 ms per 10 Runic Power. Improved Death Strike's 5 RP reduction
    // still counts as spent (SimC76f8118 sc_death_knight.cpp13183-13196).
    let expected_ms = ((paid + 5.0) / 10.0 * 2000.0) as u64;
    let reduced = before - elapsed_ms - after;
    assert!(
        reduced.abs_diff(expected_ms) <= 50,
        "reduced {reduced} ms, expected {expected_ms} ms for {paid} RP paid"
    );
}
