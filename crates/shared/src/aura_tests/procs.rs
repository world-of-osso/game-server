use super::*;

#[test]
fn proc_fires_on_matching_event() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnCrit, 1.0, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnCrit, 0.5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_spell_id, 100);
    assert_eq!(results[0].action, ProcAction::ApplySpell { spell_id: 999 });
}

#[test]
fn proc_ignores_wrong_event() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnCrit, 1.0, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.5);
    assert!(results.is_empty());
}

#[test]
fn proc_fails_roll() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnHit, 0.3, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.5);
    assert!(results.is_empty());
}

#[test]
fn proc_passes_roll() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnHit, 0.3, 0.0, 999));
    let results = auras.check_procs(ProcTrigger::OnHit, 0.2);
    assert_eq!(results.len(), 1);
}

#[test]
fn proc_icd_prevents_refire() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 10.0, 999));

    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert_eq!(results.len(), 1);

    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert!(results.is_empty());
}

#[test]
fn proc_icd_expires_after_tick() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 5.0, 999));

    auras.check_procs(ProcTrigger::OnHit, 0.0);
    auras.tick_proc_cooldowns(5.1);

    let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
    assert_eq!(results.len(), 1);
}

#[test]
fn proc_multiple_auras_checked() {
    let mut auras = Auras::default();
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 0.0, 999));
    auras
        .active
        .push(make_proc_aura(200, 2, ProcTrigger::OnHit, 1.0, 0.0, 888));
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
    auras
        .active
        .push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 10.0, 999));
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
