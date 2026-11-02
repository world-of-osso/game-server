//! Retail Piety amount for multiple overhealing sources in one tick.

#[test]
#[ignore = "requires native Priest spell data"]
fn piety_redistributes_fifteen_percent_of_multiple_overheals() {
    use crate::spell_cast::effects::{EffectCtx, apply_fixed_heal};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut h = harness(Vec::new(), &[2061, 94472, 1246802, 443126], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let full = spawn_player(&mut h.app, 10, 3.0);
    let injured = spawn_player(&mut h.app, 10, 4.0);
    *h.app.world_mut().get_mut::<Health>(injured).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1246802,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for (id, amount) in [(2061, 1000.0), (94472, 2000.0)] {
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .unwrap()
            .clone();
        let ctx = EffectCtx {
            caster: h.caster,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        apply_fixed_heal(h.app.world_mut(), &ctx, full, amount);
    }
    h.advance(10);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    let source_overheal: i32 = events
        .iter()
        .filter(|e| {
            e.kind == CombatLogKind::Heal
                && matches!(e.spell_id, Some(2061 | 94472))
                && e.target == Some(full.to_bits())
        })
        .map(|e| e.overflow)
        .sum();
    assert_eq!(source_overheal, 3000);
    let heals: Vec<_> = events
        .iter()
        .filter(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(443126)
                && e.target == Some(injured.to_bits())
        })
        .collect();
    assert_eq!(heals.len(), 1, "the pooled overheal must produce one tick");
    assert_eq!(
        heals[0].amount, 450,
        "retail redistributes 15% of both overheals"
    );
}
