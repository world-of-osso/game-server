//! Blood's passive modifies the actual Vampiric Strike proc rate.
use super::{combat, vampiric_proc};
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};
use combat::spell_cast::test_harness::harness;
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

fn proc_count(blood_passive: bool) -> usize {
    let _seed = combat::random::seed_scope(70448);
    let mut h = harness(Vec::new(), &[433901, 433899, 49998, 137008], 6).unwrap();
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(ActiveSpec(250));
    if blood_passive {
        auras::apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 137008,
                effect_mask: 1 << 12,
                source: AuraSource::Cast,
            },
        );
    }
    let strike = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(49998)
        .unwrap()
        .clone();
    let mut count = 0;
    for _ in 0..1000 {
        vampiric_proc(h.app.world_mut(), caster, &strike);
        if auras::has_aura(h.app.world(), caster, 433899) {
            count += 1;
            auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == 433899
            });
        }
    }
    count
}

#[test]
fn blood_passive_raises_vampiric_strike_proc_rate() {
    let base = proc_count(false);
    let blood = proc_count(true);
    assert!((210..290).contains(&base), "base procs: {base}");
    assert!((310..390).contains(&blood), "Blood procs: {blood}");
    assert!(blood > base, "Blood {blood}, base {base}");
}
