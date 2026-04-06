use super::*;

fn make_aura(spell_id: u32, caster: u64, max_stacks: u8) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 10.0,
        remaining: 10.0,
        max_stacks,
        ..Default::default()
    }
}

fn make_dot(spell_id: u32, caster: u64) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 15.0,
        remaining: 15.0,
        effects: [
            Some(AuraEffect::PeriodicDamage {
                damage: 100.0,
                interval: 3.0,
            }),
            None,
            None,
        ],
        tick_interval: 3.0,
        ..Default::default()
    }
}

fn make_shield(spell_id: u32, caster: u64, school: SpellSchool, absorb: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 30.0,
        remaining: 30.0,
        effects: [
            Some(AuraEffect::SchoolAbsorb { school, remaining: absorb }),
            None,
            None,
        ],
        ..Default::default()
    }
}

fn make_damage_done_buff(spell_id: u32, caster: u64, percent: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 30.0,
        remaining: 30.0,
        effects: [
            Some(AuraEffect::ModDamageDone { percent }),
            None,
            None,
        ],
        ..Default::default()
    }
}

fn make_damage_taken_debuff(spell_id: u32, caster: u64, percent: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 30.0,
        remaining: 30.0,
        effects: [
            Some(AuraEffect::ModDamageTaken { percent }),
            None,
            None,
        ],
        ..Default::default()
    }
}

fn make_proc_aura(spell_id: u32, caster: u64, trigger: ProcTrigger, chance: f32, icd: f32, triggered: u32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 60.0,
        remaining: 60.0,
        effects: [
            Some(AuraEffect::ModCritChance { percent: 0.05 }),
            None,
            None,
        ],
        proc_def: Some(ProcDef {
            trigger,
            chance,
            icd,
            icd_remaining: 0.0,
            action: ProcAction::ApplySpell { spell_id: triggered },
        }),
        ..Default::default()
    }
}

fn make_threat_aura(spell_id: u32, caster: u64, percent: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 60.0,
        remaining: 60.0,
        effects: [
            Some(AuraEffect::ModThreat { percent }),
            None,
            None,
        ],
        ..Default::default()
    }
}

fn make_cdr_aura(spell_id: u32, caster: u64, target_spell: u32, reduction: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 60.0,
        remaining: 60.0,
        effects: [
            Some(AuraEffect::ModCooldown {
                spell_id: target_spell,
                reduction,
            }),
            None,
            None,
        ],
        ..Default::default()
    }
}

fn make_speed_aura(spell_id: u32, caster: u64, percent: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent }),
            None,
            None,
        ],
        ..Default::default()
    }
}

fn make_dot_with_interval(spell_id: u32, caster: u64, damage: f32, interval: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 15.0,
        remaining: 15.0,
        effects: [
            Some(AuraEffect::PeriodicDamage { damage, interval }),
            None,
            None,
        ],
        tick_interval: interval,
        ..Default::default()
    }
}

fn make_hot(spell_id: u32, caster: u64, heal: f32, interval: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 12.0,
        remaining: 12.0,
        effects: [
            Some(AuraEffect::PeriodicHeal { heal, interval }),
            None,
            None,
        ],
        tick_interval: interval,
        ..Default::default()
    }
}

#[test]
fn aura_roundtrip_bitcode() {
    let aura = Aura {
        spell_id: 12345,
        caster: 99,
        duration: 30.0,
        remaining: 25.5,
        stacks: 2,
        max_stacks: 5,
        effects: [
            Some(AuraEffect::ModDamageDone { percent: 0.10 }),
            Some(AuraEffect::ModCritChance { percent: 0.05 }),
            None,
        ],
        ..Default::default()
    };
    let encoded = bitcode::encode(&aura);
    let decoded: Aura = bitcode::decode(&encoded).unwrap();
    assert_eq!(aura, decoded);
}

#[test]
fn auras_default_is_empty() {
    let auras = Auras::default();
    assert!(auras.active.is_empty());
}

#[test]
fn auras_collection_roundtrip_bitcode() {
    let auras = Auras {
        active: vec![
            Aura {
                spell_id: 1,
                caster: 10,
                duration: 10.0,
                remaining: 8.0,
                effects: [
                    Some(AuraEffect::PeriodicDamage {
                        damage: 500.0,
                        interval: 3.0,
                    }),
                    None,
                    None,
                ],
                tick_interval: 3.0,
                ..Default::default()
            },
            Aura {
                spell_id: 2,
                caster: 20,
                duration: 60.0,
                remaining: 45.0,
                stacks: 3,
                max_stacks: 5,
                effects: [
                    Some(AuraEffect::SchoolAbsorb {
                        school: SpellSchool::Holy,
                        remaining: 10000.0,
                    }),
                    None,
                    None,
                ],
                ..Default::default()
            },
        ],
    };
    let encoded = bitcode::encode(&auras);
    let decoded: Auras = bitcode::decode(&encoded).unwrap();
    assert_eq!(auras, decoded);
}

#[test]
fn aura_effect_variants() {
    // Verify all variants can be constructed
    let effects: Vec<AuraEffect> = vec![
        AuraEffect::ModDamageDone { percent: 0.1 },
        AuraEffect::ModDamageTaken { percent: -0.2 },
        AuraEffect::SchoolAbsorb { school: SpellSchool::Fire, remaining: 5000.0 },
        AuraEffect::PeriodicDamage { damage: 100.0, interval: 3.0 },
        AuraEffect::PeriodicHeal { heal: 200.0, interval: 2.0 },
        AuraEffect::ModStat { stamina: 50.0, strength: 0.0, agility: 0.0, intellect: 0.0, spirit: 0.0 },
        AuraEffect::ModAttackSpeed { percent: 0.2 },
        AuraEffect::ModCastSpeed { percent: 0.15 },
        AuraEffect::ModHaste { percent: 0.1 },
        AuraEffect::ModCritChance { percent: 0.05 },
        AuraEffect::ModThreat { percent: 0.43 },
        AuraEffect::ModCooldown { spell_id: 100, reduction: 2.0 },
        AuraEffect::ModMovementSpeed { percent: -0.50 },
    ];
    assert_eq!(effects.len(), 13);
}

#[test]
fn aura_has_periodic_effect_dot() {
    let dot = make_dot(1, 10);
    assert!(dot.has_periodic_effect());
}

#[test]
fn aura_has_periodic_effect_buff() {
    let buff = make_aura(1, 10, 1);
    assert!(!buff.has_periodic_effect());
}

#[test]
fn apply_aura_first_time() {
    let mut auras = Auras::default();
    let result = auras.apply(make_aura(100, 1, 3));
    assert_eq!(result, AuraApplyResult::Applied);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn apply_aura_same_caster_refreshes_duration() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 3));
    // Simulate time passing
    auras.active[0].remaining = 3.0;

    let result = auras.apply(make_aura(100, 1, 3));
    assert_eq!(result, AuraApplyResult::Refreshed);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].remaining, 10.0, "duration should reset");
}

#[test]
fn apply_aura_same_caster_increments_stacks() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 5));
    assert_eq!(auras.active[0].stacks, 1);

    auras.apply(make_aura(100, 1, 5));
    assert_eq!(auras.active[0].stacks, 2);

    auras.apply(make_aura(100, 1, 5));
    assert_eq!(auras.active[0].stacks, 3);
}

#[test]
fn apply_aura_same_caster_caps_at_max_stacks() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 2));
    auras.apply(make_aura(100, 1, 2));
    auras.apply(make_aura(100, 1, 2));
    assert_eq!(auras.active[0].stacks, 2, "should not exceed max_stacks");
}

#[test]
fn apply_aura_different_caster_non_periodic_rejected() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let result = auras.apply(make_aura(100, 2, 1));
    assert_eq!(result, AuraApplyResult::Rejected);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].caster, 1, "original caster kept");
}

#[test]
fn apply_dot_different_casters_stack() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    let result = auras.apply(make_dot(200, 2));
    assert_eq!(result, AuraApplyResult::Applied);
    assert_eq!(auras.active.len(), 2, "DOTs from different casters stack");
}

#[test]
fn apply_dot_same_caster_refreshes() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    auras.active[0].remaining = 5.0;

    let result = auras.apply(make_dot(200, 1));
    assert_eq!(result, AuraApplyResult::Refreshed);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].remaining, 15.0);
}

#[test]
fn apply_different_spells_always_stack() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let result = auras.apply(make_aura(200, 1, 1));
    assert_eq!(result, AuraApplyResult::Applied);
    assert_eq!(auras.active.len(), 2);
}

#[test]
fn tick_expires_aura_at_zero() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1)); // 10s duration
    let expired = auras.tick_and_expire(10.1);
    assert_eq!(expired, 1);
    assert!(auras.active.is_empty());
}

#[test]
fn tick_does_not_expire_with_time_left() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let expired = auras.tick_and_expire(5.0);
    assert_eq!(expired, 0);
    assert_eq!(auras.active.len(), 1);
    assert!((auras.active[0].remaining - 5.0).abs() < 0.01);
}

#[test]
fn tick_expires_some_keeps_others() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1)); // 10s
    auras.apply(make_dot(200, 2)); // 15s
    let expired = auras.tick_and_expire(12.0);
    assert_eq!(expired, 1);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].spell_id, 200, "DOT should survive");
}

#[test]
fn dispel_removes_all_instances_of_spell() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    auras.apply(make_dot(200, 2)); // different caster DOT
    auras.apply(make_aura(300, 1, 1));
    let removed = auras.dispel(200);
    assert_eq!(removed, 2);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].spell_id, 300);
}

#[test]
fn dispel_nonexistent_spell_removes_nothing() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let removed = auras.dispel(999);
    assert_eq!(removed, 0);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn cancel_removes_specific_caster_aura() {
    let mut auras = Auras::default();
    auras.apply(make_dot(200, 1));
    auras.apply(make_dot(200, 2));
    let removed = auras.cancel(200, 1);
    assert!(removed);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].caster, 2, "other caster's DOT kept");
}

#[test]
fn cancel_wrong_caster_returns_false() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    let removed = auras.cancel(100, 999);
    assert!(!removed);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn clear_all_removes_everything() {
    let mut auras = Auras::default();
    auras.apply(make_aura(100, 1, 1));
    auras.apply(make_dot(200, 2));
    auras.apply(make_aura(300, 3, 5));
    auras.clear_all();
    assert!(auras.active.is_empty());
}

#[test]
fn absorb_fully_absorbed() {
    let mut auras = Auras::default();
    auras.active.push(make_shield(100, 1, SpellSchool::Fire, 500.0));
    let leftover = auras.absorb_damage(300.0, SpellSchool::Fire);
    assert_eq!(leftover, 0.0);
    if let Some(AuraEffect::SchoolAbsorb { remaining, .. }) =
        auras.active[0].effects[0]
    {
        assert!((remaining - 200.0).abs() < 0.01);
    } else {
        panic!("shield effect missing");
    }
}

#[test]
fn absorb_partially_absorbed() {
    let mut auras = Auras::default();
    auras.active.push(make_shield(100, 1, SpellSchool::Fire, 200.0));
    let leftover = auras.absorb_damage(500.0, SpellSchool::Fire);
    assert!((leftover - 300.0).abs() < 0.01);
    assert!(auras.active.is_empty());
}

#[test]
fn absorb_wrong_school_ignored() {
    let mut auras = Auras::default();
    auras.active.push(make_shield(100, 1, SpellSchool::Fire, 500.0));
    let leftover = auras.absorb_damage(300.0, SpellSchool::Frost);
    assert_eq!(leftover, 300.0);
    assert_eq!(auras.active.len(), 1);
}

#[test]
fn absorb_weakest_first() {
    let mut auras = Auras::default();
    auras.active.push(make_shield(100, 1, SpellSchool::Shadow, 1000.0));
    auras.active.push(make_shield(200, 2, SpellSchool::Shadow, 200.0));

    let leftover = auras.absorb_damage(300.0, SpellSchool::Shadow);
    assert_eq!(leftover, 0.0);
    assert_eq!(auras.active.len(), 1, "weak shield should be depleted");
    assert_eq!(auras.active[0].spell_id, 100);
    if let Some(AuraEffect::SchoolAbsorb { remaining, .. }) =
        auras.active[0].effects[0]
    {
        assert!((remaining - 900.0).abs() < 0.01);
    } else {
        panic!("strong shield effect missing");
    }
}

#[test]
fn absorb_multiple_shields_chained() {
    let mut auras = Auras::default();
    auras.active.push(make_shield(100, 1, SpellSchool::Holy, 100.0));
    auras.active.push(make_shield(200, 2, SpellSchool::Holy, 150.0));

    let leftover = auras.absorb_damage(400.0, SpellSchool::Holy);
    assert!((leftover - 150.0).abs() < 0.01);
    assert!(auras.active.is_empty());
}

#[test]
fn absorb_zero_damage_returns_zero() {
    let mut auras = Auras::default();
    auras.active.push(make_shield(100, 1, SpellSchool::Fire, 500.0));
    let leftover = auras.absorb_damage(0.0, SpellSchool::Fire);
    assert_eq!(leftover, 0.0);
}

#[test]
fn absorb_preserves_non_absorb_auras() {
    let mut auras = Auras::default();
    let mut buff = make_aura(300, 1, 1);
    buff.effects[0] = Some(AuraEffect::ModDamageDone { percent: 0.10 });
    auras.apply(buff);
    auras.active.push(make_shield(100, 2, SpellSchool::Fire, 50.0));
    let leftover = auras.absorb_damage(50.0, SpellSchool::Fire);
    assert_eq!(leftover, 0.0);
    assert_eq!(auras.active.len(), 1);
    assert_eq!(auras.active[0].spell_id, 300);
}

#[test]
fn damage_done_no_auras() {
    let auras = Auras::default();
    assert_eq!(auras.damage_done_multiplier(), 1.0);
}

#[test]
fn damage_done_single_buff() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_done_buff(100, 1, 0.10));
    assert!((auras.damage_done_multiplier() - 1.10).abs() < 0.001);
}

#[test]
fn damage_done_multiplicative_stacking() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_done_buff(100, 1, 0.10)); // +10%
    auras.active.push(make_damage_done_buff(200, 2, 0.20)); // +20%
    // 1.1 * 1.2 = 1.32, NOT 1.3 (additive)
    assert!((auras.damage_done_multiplier() - 1.32).abs() < 0.001);
}

#[test]
fn damage_done_negative_reduces() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_done_buff(100, 1, -0.20)); // -20%
    assert!((auras.damage_done_multiplier() - 0.80).abs() < 0.001);
}

#[test]
fn damage_taken_single_debuff() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_taken_debuff(100, 1, 0.20));
    assert!((auras.damage_taken_multiplier() - 1.20).abs() < 0.001);
}

#[test]
fn damage_taken_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_taken_debuff(100, 1, -0.10));
    assert!((auras.damage_taken_multiplier() - 0.90).abs() < 0.001);
}

#[test]
fn damage_taken_multiplicative_stacking() {
    let mut auras = Auras::default();
    auras.active.push(make_damage_taken_debuff(100, 1, -0.10));
    auras.active.push(make_damage_taken_debuff(200, 2, -0.20));
    // 0.9 * 0.8 = 0.72
    assert!((auras.damage_taken_multiplier() - 0.72).abs() < 0.001);
}

#[test]
fn damage_modifiers_ignore_other_effects() {
    let mut auras = Auras::default();
    auras.active.push(make_dot(100, 1));
    auras.active.push(make_shield(200, 1, SpellSchool::Fire, 500.0));
    assert_eq!(auras.damage_done_multiplier(), 1.0);
    assert_eq!(auras.damage_taken_multiplier(), 1.0);
}

#[test]
fn proc_fires_on_matching_event() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnCrit, 1.0, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnCrit, 0.5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_spell_id, 100);
    assert_eq!(results[0].action, ProcAction::ApplySpell { spell_id: 999 });
}

#[test]
fn proc_ignores_wrong_event() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnCrit, 1.0, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.5);
    assert!(results.is_empty());
}

#[test]
fn proc_fails_roll() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 0.3, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.5);
    assert!(results.is_empty());
}

#[test]
fn proc_passes_roll() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 0.3, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.2);
    assert_eq!(results.len(), 1);
}

#[test]
fn proc_icd_prevents_refire() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 10.0, 999));

    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert_eq!(results.len(), 1);

    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert!(results.is_empty());
}

#[test]
fn proc_icd_expires_after_tick() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 5.0, 999));

    auras.check_procs(ProcTrigger::OnHit, 0.0);
    auras.tick_proc_cooldowns(5.1);

    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert_eq!(results.len(), 1);
}

#[test]
fn proc_multiple_auras_checked() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 0.0, 999));
    auras.active.push(make_proc_aura(200, 2, ProcTrigger::OnHit, 1.0, 0.0, 888));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert_eq!(results.len(), 2);
}

#[test]
fn proc_no_proc_def_skipped() {
    let mut auras = Auras::default();
    auras.active.push(make_aura(100, 1, 1));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert!(results.is_empty());
}

#[test]
fn tick_proc_cooldowns_decrements() {
    let mut auras = Auras::default();
    auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 10.0, 999));
    auras.check_procs(ProcTrigger::OnHit, 0.0);
    auras.tick_proc_cooldowns(3.0);
    let icd = auras.active[0].proc_def.as_ref().unwrap().icd_remaining;
    assert!((icd - 7.0).abs() < 0.01);
}

#[test]
fn proc_cooldown_reset() {
    use crate::casting::SpellCooldowns;
    let aura = Aura {
        spell_id: 500,
        caster: 1,
        duration: 60.0,
        remaining: 60.0,
        effects: [None, None, None],
        proc_def: Some(ProcDef {
            trigger: ProcTrigger::OnCrit,
            chance: 1.0,
            icd: 0.0,
            icd_remaining: 0.0,
            action: ProcAction::ResetCooldown { spell_id: 47486 },
        }),
        ..Default::default()
    };
    let mut auras = Auras::default();
    auras.active.push(aura);

    let results = auras.check_procs(ProcTrigger::OnCrit, 0.0);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].action,
        ProcAction::ResetCooldown { spell_id: 47486 }
    );

    let mut cds = SpellCooldowns::default();
    cds.start(47486, 6.0);
    assert!(cds.is_on_cooldown(47486));

    if let ProcAction::ResetCooldown { spell_id } = results[0].action {
        cds.reset(spell_id);
    }
    assert!(!cds.is_on_cooldown(47486));
}

#[test]
fn proc_action_variants() {
    let apply = ProcAction::ApplySpell { spell_id: 100 };
    let reset = ProcAction::ResetCooldown { spell_id: 200 };
    assert_ne!(apply, reset);
}

#[test]
fn threat_multiplier_no_auras() {
    let auras = Auras::default();
    assert_eq!(auras.threat_multiplier(), 1.0);
}

#[test]
fn threat_multiplier_righteous_fury() {
    let mut auras = Auras::default();
    auras.active.push(make_threat_aura(100, 1, 0.43));
    assert!((auras.threat_multiplier() - 1.43).abs() < 0.001);
}

#[test]
fn threat_multiplier_stacks_multiplicatively() {
    let mut auras = Auras::default();
    auras.active.push(make_threat_aura(100, 1, 0.30));
    auras.active.push(make_threat_aura(200, 1, 0.10));
    assert!((auras.threat_multiplier() - 1.43).abs() < 0.001);
}

#[test]
fn threat_multiplier_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_threat_aura(100, 1, -0.30));
    assert!((auras.threat_multiplier() - 0.70).abs() < 0.001);
}

#[test]
fn cdr_no_auras_returns_base() {
    let auras = Auras::default();
    assert_eq!(auras.effective_cooldown(100, 10.0), 10.0);
}

#[test]
fn cdr_specific_spell_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 100, 2.0));
    assert_eq!(auras.effective_cooldown(100, 10.0), 8.0);
    assert_eq!(auras.effective_cooldown(200, 10.0), 10.0);
}

#[test]
fn cdr_global_reduction() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 0, 1.0));
    assert_eq!(auras.effective_cooldown(100, 8.0), 7.0);
    assert_eq!(auras.effective_cooldown(200, 5.0), 4.0);
}

#[test]
fn cdr_stacks_additively() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 100, 2.0));
    auras.active.push(make_cdr_aura(1001, 1, 100, 1.5));
    assert!((auras.effective_cooldown(100, 10.0) - 6.5).abs() < 0.001);
}

#[test]
fn cdr_clamps_to_zero() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 100, 20.0));
    assert_eq!(auras.effective_cooldown(100, 8.0), 0.0);
}

#[test]
fn cdr_global_and_specific_stack() {
    let mut auras = Auras::default();
    auras.active.push(make_cdr_aura(1000, 1, 0, 1.0));
    auras.active.push(make_cdr_aura(1001, 1, 100, 2.0));
    assert_eq!(auras.effective_cooldown(100, 10.0), 7.0);
    assert_eq!(auras.effective_cooldown(200, 10.0), 9.0);
}

#[test]
fn speed_no_auras_normal() {
    let auras = Auras::default();
    assert_eq!(auras.movement_speed_multiplier(), 1.0);
    assert!(!auras.is_rooted());
}

#[test]
fn speed_snare_50_percent() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -0.50));
    assert!((auras.movement_speed_multiplier() - 0.50).abs() < 0.001);
}

#[test]
fn speed_root_is_zero() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -1.0));
    assert_eq!(auras.movement_speed_multiplier(), 0.0);
    assert!(auras.is_rooted());
}

#[test]
fn speed_strongest_snare_wins() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -0.30));
    auras.active.push(make_speed_aura(200, 2, -0.70));
    assert!((auras.movement_speed_multiplier() - 0.30).abs() < 0.001);
}

#[test]
fn speed_buff_stacks_additively() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, 0.30));
    auras.active.push(make_speed_aura(200, 2, 0.20));
    assert!((auras.movement_speed_multiplier() - 1.50).abs() < 0.001);
}

#[test]
fn speed_buff_and_snare_combined() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -0.50));
    auras.active.push(make_speed_aura(200, 2, 0.20));
    assert!((auras.movement_speed_multiplier() - 0.70).abs() < 0.001);
}

#[test]
fn speed_root_overrides_buff() {
    let mut auras = Auras::default();
    auras.active.push(make_speed_aura(100, 1, -1.0));
    auras.active.push(make_speed_aura(200, 2, 0.50));
    assert_eq!(auras.movement_speed_multiplier(), 0.0);
}

#[test]
fn periodic_tick_fires_at_interval() {
    let mut dot = make_dot_with_interval(100, 1, 200.0, 3.0);
    let ticks = dot.tick_periodic(2.0);
    assert!(ticks.is_empty());
    let ticks = dot.tick_periodic(1.5);
    assert_eq!(ticks, vec![PeriodicTick::Damage(200.0)]);
}

#[test]
fn periodic_tick_fires_multiple_on_catchup() {
    let mut dot = make_dot_with_interval(100, 1, 150.0, 2.0);
    let ticks = dot.tick_periodic(5.0);
    assert_eq!(ticks, vec![PeriodicTick::Damage(150.0), PeriodicTick::Damage(150.0)]);
    assert!((dot.tick_timer - 1.0).abs() < 0.001);
}

#[test]
fn periodic_hot_produces_heal_ticks() {
    let mut hot = make_hot(200, 1, 500.0, 2.0);
    let ticks = hot.tick_periodic(2.5);
    assert_eq!(ticks, vec![PeriodicTick::Heal(500.0)]);
}

#[test]
fn periodic_no_tick_if_interval_zero() {
    let mut aura = make_aura(100, 1, 1);
    let ticks = aura.tick_periodic(10.0);
    assert!(ticks.is_empty());
}

#[test]
fn tick_all_periodic_aggregates_across_auras() {
    let mut auras = Auras::default();
    auras.active.push(make_dot_with_interval(100, 1, 100.0, 2.0));
    auras.active.push(make_hot(200, 2, 300.0, 2.0));

    let ticks = auras.tick_all_periodic(2.5);
    assert_eq!(ticks.len(), 2);
    assert_eq!(ticks[0], (1, PeriodicTick::Damage(100.0)));
    assert_eq!(ticks[1], (2, PeriodicTick::Heal(300.0)));
}

#[test]
fn snapshot_stats_preserved_on_aura() {
    let aura = Aura {
        spell_id: 100,
        caster: 1,
        duration: 15.0,
        remaining: 15.0,
        effects: [
            Some(AuraEffect::PeriodicDamage { damage: 200.0, interval: 3.0 }),
            None,
            None,
        ],
        tick_interval: 3.0,
        snapshot_spell_power: 3500.0,
        snapshot_attack_power: 1200.0,
        ..Default::default()
    };
    assert_eq!(aura.snapshot_spell_power, 3500.0);
    assert_eq!(aura.snapshot_attack_power, 1200.0);
}

#[test]
fn haste_reduces_tick_interval_at_application() {
    use crate::formulas::spell::hasted_periodic;
    let haste = hasted_periodic(3.0, 4, 0.5);
    let aura = Aura {
        spell_id: 100,
        caster: 1,
        duration: haste.duration,
        remaining: haste.duration,
        effects: [
            Some(AuraEffect::PeriodicDamage { damage: 100.0, interval: 3.0 }),
            None,
            None,
        ],
        tick_interval: haste.tick_interval,
        ..Default::default()
    };
    assert!((aura.tick_interval - 2.0).abs() < 0.001);
    assert_eq!(haste.num_ticks, 6);
}
