//! SimC76f8118 sc_shaman.cpp12600–12608/12636–12651/12914–12924:
//! earned MID2 builder uses and the actual free-spender lifecycle.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use super::*;
use shared::components::PowerType;

const SET_BONUS: u32 = 1296626;
const STORMKEEPER: u32 = 191634;
const ASCENDANCE: u32 = 1219480;
const FLOWING: u32 = 1300219;
const OVERCHARGE: u32 = 1300222;
const LIGHTNING_BOLT: u32 = 188196;
const ELEMENTAL_BLAST: u32 = 117014;

fn cast_self(run: &mut Run, spell: u32) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(run);
}

fn spend_stormkeeper(run: &mut Run) {
    cast_self(run, STORMKEEPER);
    let uses = own_aura(run, STORMKEEPER)
        .expect("actual native Stormkeeper")
        .stacks;
    assert_eq!(uses, 2, "native70448 Stormkeeper uses");
    for _ in 0..uses {
        cast(run, LIGHTNING_BOLT);
    }
    assert!(
        own_aura(run, STORMKEEPER).is_none(),
        "ordinary roots spent SK"
    );
}

fn power(run: &Run) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Maelstrom)
        .expect("native Maelstrom")
        .0
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_tier_native_stormkeeper_earns_two_builder_uses_and_free_spender() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(
        own_aura(&run, SET_BONUS).is_some(),
        "actually equipped native MID2 four-piece"
    );
    spend_stormkeeper(&mut run);
    let flowing = own_aura(&run, FLOWING).expect("last actual SK use earns Flowing Elements");
    assert_eq!(flowing.stacks, 2, "two earned native builder uses");
    assert_eq!(
        flowing
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount,
        25.0
    );
    assert!(
        own_aura(&run, OVERCHARGE).is_none(),
        "no free spender before a builder use"
    );
    for _ in 0..4 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            LIGHTNING_BOLT,
            Some(run.targets[0]),
            1,
        ));
    }
    assert_eq!(
        own_aura(&run, FLOWING).unwrap().stacks,
        2,
        "background builders do not spend uses"
    );
    cast(&mut run, LIGHTNING_BOLT);
    assert_eq!(own_aura(&run, FLOWING).unwrap().stacks, 1);
    assert!(
        own_aura(&run, OVERCHARGE).is_some(),
        "ordinary builder earns actual free-spender aura"
    );
    let before = power(&run);
    cast(&mut run, ELEMENTAL_BLAST);
    assert_eq!(
        power(&run),
        before,
        "native Overcharge makes Blast free without granting Maelstrom"
    );
    assert!(
        own_aura(&run, OVERCHARGE).is_none(),
        "ordinary Blast consumes Overcharge"
    );
    assert_eq!(
        own_aura(&run, FLOWING).unwrap().stacks,
        1,
        "spender does not consume another builder use"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_tier_native_stormkeeper_inside_ascendance_defers_four_builder_uses() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(own_aura(&run, SET_BONUS).is_some());
    cast_self(&mut run, ASCENDANCE);
    spend_stormkeeper(&mut run);
    assert!(
        own_aura(&run, ASCENDANCE).is_some(),
        "SK roots completed inside actual Ascendance"
    );
    assert!(
        own_aura(&run, FLOWING).is_none(),
        "overlap buffers the earned tier uses"
    );
    for _ in 0..500 {
        run.app.update();
        if own_aura(&run, ASCENDANCE).is_none() {
            break;
        }
    }
    assert!(
        own_aura(&run, ASCENDANCE).is_none(),
        "actual Ascendance expired"
    );
    assert_eq!(
        own_aura(&run, FLOWING)
            .expect("actual Ascendance expiry releases buffer")
            .stacks,
        4
    );
}
