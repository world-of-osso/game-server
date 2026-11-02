use crate::combat::{auras, spell_cast};
use crate::test_support::harness;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
fn merektha_thunder_spit_marker_expires_into_repeated_native_damage() {
    let mut h = harness(Vec::new(), &[1289109, 1289588, 1298329], 1).unwrap();
    let attacker = h.npc;
    let target = h.caster;
    let mut health = h.app.world_mut().get_mut::<Health>(target).unwrap();
    health.max = 1_000_000_000.0;
    health.current = health.max;
    auras::apply_aura(
        h.app.world_mut(),
        &auras::AuraApplication {
            caster: attacker,
            target,
            spell_id: 1289109,
            effect_mask: 1,
            source: auras::AuraSource::Cast,
        },
    );
    h.advance(150);
    let damage = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(1298329)
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        damage, 5,
        "the marker must produce an immediate strike and four periodic strikes"
    );
    assert!(!auras::has_aura(h.app.world(), target, 1289588));
}

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
