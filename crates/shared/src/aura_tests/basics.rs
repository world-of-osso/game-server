use super::*;

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
    let effects: Vec<AuraEffect> = vec![
        AuraEffect::ModDamageDone { percent: 0.1 },
        AuraEffect::ModDamageTaken { percent: -0.2 },
        AuraEffect::SchoolAbsorb {
            school: SpellSchool::Fire,
            remaining: 5000.0,
        },
        AuraEffect::PeriodicDamage {
            damage: 100.0,
            interval: 3.0,
        },
        AuraEffect::PeriodicHeal {
            heal: 200.0,
            interval: 2.0,
        },
        AuraEffect::ModStat {
            stamina: 50.0,
            strength: 0.0,
            agility: 0.0,
            intellect: 0.0,
            spirit: 0.0,
        },
        AuraEffect::ModAttackSpeed { percent: 0.2 },
        AuraEffect::ModCastSpeed { percent: 0.15 },
        AuraEffect::ModHaste { percent: 0.1 },
        AuraEffect::ModCritChance { percent: 0.05 },
        AuraEffect::ModThreat { percent: 0.43 },
        AuraEffect::ModCooldown {
            spell_id: 100,
            reduction: 2.0,
        },
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
