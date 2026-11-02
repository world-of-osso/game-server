use crate::combat::{auras, spell_cast};
use crate::test_support::harness;
use shared::components::Health;

#[test]
fn temple_strikes_apply_their_native_debuffs_on_hit() {
    for (strike, debuff) in [(1290797, 1308838), (1300803, 1303446)] {
        let mut h = harness(Vec::new(), &[strike, debuff], 1).unwrap();
        let victim = h.caster;
        let mut health = h.app.world_mut().get_mut::<Health>(victim).unwrap();
        health.max = 1_000_000_000.0;
        health.current = health.max;
        spell_cast::creature_cast(h.app.world_mut(), h.npc, strike, Some(victim), false).unwrap();
        h.advance(100);
        assert!(
            auras::has_aura(h.app.world(), victim, debuff),
            "strike {strike} must apply {debuff} to its actual victim"
        );
    }
}
