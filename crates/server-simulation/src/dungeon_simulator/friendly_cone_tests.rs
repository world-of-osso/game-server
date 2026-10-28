use crate::spell_cast::test_harness::{harness, spawn_player};

#[test]
#[ignore = "requires native Dream Breath spell data"]
fn native_friendly_cone_selects_front_allies_not_back_or_enemies() {
    let mut h = harness(Vec::new(), &[355941], 13).unwrap();
    let front = spawn_player(&mut h.app, 13, 10.0);
    let back = spawn_player(&mut h.app, 13, -10.0);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(355941)
        .unwrap()
        .clone();
    let mut effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .clone();
    // Isolate selector59 from the second, destination-area selector in this native effect.
    effect.implicit_targets[1] = 0;
    let units = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        h.caster,
        None,
        &spell,
        &effect,
        0,
        false,
    )
    .unwrap();
    assert!(units.contains(&front), "{units:?}");
    assert!(!units.contains(&back), "{units:?}");
    assert!(!units.contains(&h.npc), "{units:?}");
}
