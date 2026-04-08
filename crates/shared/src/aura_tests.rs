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
            Some(AuraEffect::SchoolAbsorb {
                school,
                remaining: absorb,
            }),
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
        effects: [Some(AuraEffect::ModDamageDone { percent }), None, None],
        ..Default::default()
    }
}

fn make_damage_taken_debuff(spell_id: u32, caster: u64, percent: f32) -> Aura {
    Aura {
        spell_id,
        caster,
        duration: 30.0,
        remaining: 30.0,
        effects: [Some(AuraEffect::ModDamageTaken { percent }), None, None],
        ..Default::default()
    }
}

fn make_proc_aura(
    spell_id: u32,
    caster: u64,
    trigger: ProcTrigger,
    chance: f32,
    icd: f32,
    triggered: u32,
) -> Aura {
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
            action: ProcAction::ApplySpell {
                spell_id: triggered,
            },
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
        effects: [Some(AuraEffect::ModThreat { percent }), None, None],
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
        effects: [Some(AuraEffect::ModMovementSpeed { percent }), None, None],
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

#[path = "aura_tests/absorption.rs"]
mod absorption;
#[path = "aura_tests/basics.rs"]
mod basics;
#[path = "aura_tests/lifecycle.rs"]
mod lifecycle;
#[path = "aura_tests/modifiers.rs"]
mod modifiers;
#[path = "aura_tests/periodic.rs"]
mod periodic;
#[path = "aura_tests/procs.rs"]
mod procs;
