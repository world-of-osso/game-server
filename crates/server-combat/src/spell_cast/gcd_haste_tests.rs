//! Native shared GCD clocks: representative foreground actions for all40 specs.
use super::test_harness::*;
use super::*;
use shared::components::ActiveSpec;

// Native70448 category133 records. `hasted` describes the ordinary spell path,
// without a learned417 passive; that independent path has existing aura tests.
const ACTIONS: &[(u32, u8, u32, bool, u32)] = &[
    (62, 8, 44425, true, 1500),
    (63, 8, 133, true, 1500),
    (64, 8, 30455, true, 1500),
    (65, 2, 19750, true, 1500),
    (66, 2, 31935, true, 1500),
    (70, 2, 383328, false, 1500),
    (71, 1, 163201, false, 1500),
    (72, 1, 184367, false, 1500),
    (73, 1, 23922, false, 1500),
    (102, 11, 5176, true, 1500),
    (103, 11, 5221, false, 1000),
    (104, 11, 33917, false, 1500),
    (105, 11, 774, true, 1500),
    (250, 6, 206930, false, 1500),
    (251, 6, 49020, false, 1500),
    (252, 6, 85948, false, 1500),
    (253, 3, 193455, false, 1500),
    (254, 3, 185358, false, 1500),
    (255, 3, 259491, false, 1500),
    (256, 5, 585, true, 1500),
    (257, 5, 2061, true, 1500),
    (258, 5, 335467, true, 1500),
    (259, 4, 1329, false, 1000),
    (260, 4, 193315, false, 1000),
    (261, 4, 196819, false, 1000),
    (262, 7, 188196, true, 1500),
    (263, 7, 17364, false, 1500),
    (264, 7, 8004, true, 1500),
    (265, 9, 686, true, 1500),
    (266, 9, 264178, true, 1500),
    (267, 9, 29722, true, 1500),
    (268, 10, 100780, false, 1500),
    (269, 10, 100784, false, 1500),
    (270, 10, 116670, true, 1500),
    (577, 12, 162243, false, 1500),
    (581, 12, 228477, false, 1500),
    (1467, 13, 361469, true, 1500),
    (1468, 13, 361469, true, 1500),
    (1473, 13, 361469, true, 1500),
    (1480, 12, 1217610, true, 1500),
];

#[test]
fn ordinary_spell_gcd_haste_all_specs_native_clock_boundaries() {
    // TC a352b1fa Spell.cpp9209–9224; SimC76f8118 action.cpp648–666,
    //1285–1315: ordinary spells haste their GCD, abilities keep their own policy.
    for &(spec, class, id, hasted, base_ms) in ACTIONS {
        let mut h = harness(Vec::new(), &[id], class).expect("native GCD fixture");
        h.advance(3);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ActiveSpec(spec));
        let spell = world_db().unwrap().store.get(id).unwrap();
        for haste in [0.0, 25.0, 150.0] {
            h.app
                .world_mut()
                .get_mut::<CharacterStats>(h.caster)
                .unwrap()
                .haste_pct = haste;
            start_gcd(h.app.world_mut(), h.caster, spell);
            let expected_ms = if hasted {
                ((base_ms as f32 / (1.0 + haste / 100.0)) as u32).clamp(750, 1500)
            } else {
                base_ms
            };
            let boundary = history_now(h.app.world()) + f64::from(expected_ms) / 1000.0;
            let history = h.app.world().get::<SpellHistory>(h.caster).unwrap();
            assert!(
                history.on_gcd(spell, boundary - 0.001),
                "spec{spec} spell{id} haste{haste}: GCD ended early"
            );
            assert!(
                !history.on_gcd(spell, boundary + 0.001),
                "spec{spec} spell{id} haste{haste}: GCD exceeded{expected_ms}ms"
            );
        }
    }
}
