//! Piety excludes summon overhealing but retains class followers.

#[test]
#[ignore = "requires native Priest spell data"]
fn piety_excludes_pet_overheal_but_includes_follower_overheal() {
    use crate::spell_cast::effects::{EffectCtx, apply_fixed_heal};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut h = harness(Vec::new(), &[2061, 1246802, 443126], 10).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let player = spawn_player(&mut h.app, 10, 3.0);
    let follower = spawn_player(&mut h.app, 10, 4.0);
    let pet = spawn_player(&mut h.app, 10, 5.0);
    let injured = spawn_player(&mut h.app, 10, 6.0);
    for unit in [follower, pet] {
        h.app
            .world_mut()
            .entity_mut(unit)
            .insert(crate::zones::TempSummon { summoner: h.caster });
    }
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert(crate::followers::Follower {
            owner: h.caster,
            profile: crate::followers::Profile {
                entry: 1,
                name: "Follower".into(),
                class_id: 5,
                spec_id: 257,
                role: shared::group::GroupRole::Dps,
                attack_spell: 0,
                cooldown_spell: 0,
                heal_spell: 0,
                taunt_spell: 0,
                form_spell: 0,
                melee: false,
            },
        });
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
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(2061)
        .unwrap()
        .clone();
    let ctx = EffectCtx {
        caster: h.caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    for (target, amount) in [(player, 1000.0), (follower, 1000.0), (pet, 5000.0)] {
        apply_fixed_heal(h.app.world_mut(), &ctx, target, amount);
    }
    h.advance(10);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    let piety: Vec<_> = events
        .iter()
        .filter(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(443126)
                && e.target == Some(injured.to_bits())
        })
        .collect();
    assert_eq!(piety.len(), 1);
    assert_eq!(
        piety[0].amount, 300,
        "15% of player and follower overheal only"
    );
}
