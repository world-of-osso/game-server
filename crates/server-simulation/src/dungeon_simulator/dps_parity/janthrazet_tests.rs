//! Native equipped Jan'thrazet, the Soul Fang: actual spells proc its haste buff.
use super::*;
use crate::auras::ActiveAuras;

const DRIVER: u32 = 1_298_085;
const SOUL_FANG_ALACRITY: u32 = 1_305_360;

#[test]
#[ignore = "requires private265 profile wearing native271092 and native70448 world DB"]
fn janthrazet_native_spells_grant_item_scaled_soul_fang_alacrity() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(265, 1).unwrap();
    let actor = run.actor;
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    let driver = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == DRIVER && aura.caster == actor)
        .expect("actual native equipped Jan'thrazet driver")
        .clone();
    let haste = driver
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    assert!(
        haste > 0.0,
        "native item scaling supplies driver effect2 haste {haste}"
    );
    engage(&mut run);
    for _ in 0..TICKS {
        run.app.update();
        drive_player(&mut run).unwrap();
        let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
        let Some(buff) = auras
            .iter()
            .find(|aura| aura.spell_id == SOUL_FANG_ALACRITY && aura.caster == actor)
        else {
            continue;
        };
        // SimC76f8118 unique_gear_midnight.cpp5209–5217: the driver's effect2
        // item-scaled value becomes Soul Fang Alacrity's haste rating.
        let amount = buff
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount;
        assert!(
            (amount - haste).abs() < 0.001,
            "Soul Fang Alacrity must use driver haste {haste}, got {amount}"
        );
        assert_eq!(buff.cast_item_level, driver.cast_item_level);
        return;
    }
    panic!("native spells never applied Soul Fang Alacrity{SOUL_FANG_ALACRITY}");
}
