//! Register new modules with one alphabetical line. The numeric position preserves
//! historical dispatch order independently of declaration order; do not renumber it.
use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;

macro_rules! register_modules {
    ($($name:ident: ($position:literal, $path:literal)),* $(,)?) => {
        $(#[path = $path] mod $name;)*
        pub const MODULES: &[&[SpellScriptHook]] = &order_modules([
            $(($position, $name::HOOKS)),*
        ]);
    };
}

const fn order_modules<const N: usize>(
    mut modules: [(u16, &'static [SpellScriptHook]); N],
) -> [&'static [SpellScriptHook]; N] {
    let mut index = 1;
    while index < N {
        let entry = modules[index];
        let mut position = index;
        while position > 0 && modules[position - 1].0 > entry.0 {
            modules[position] = modules[position - 1];
            position -= 1;
        }
        modules[position] = entry;
        index += 1;
    }
    let mut ordered: [&[SpellScriptHook]; N] = [&[]; N];
    index = 0;
    while index < N {
        if index > 0 {
            assert!(
                modules[index - 1].0 != modules[index].0,
                "duplicate module position"
            );
        }
        ordered[index] = modules[index].1;
        index += 1;
    }
    ordered
}

register_modules! {
    affliction_agony: (2650, "modules/affliction_agony.rs"),
    after_cast_auras: (100, "modules/after_cast_auras.rs"),
    assassination_target: (2590, "modules/assassination_target.rs"),
    aura_rolls: (110, "modules/aura_rolls.rs"),
    beast_mastery: (20, "modules/beast_mastery.rs"),
    cast_grants: (120, "modules/cast_grants.rs"),
    cast_relationships: (370, "modules/cast_relationships.rs"),
    cast_resets: (90, "modules/cast_resets.rs"),
    destruction_periodic: (2670, "modules/destruction_periodic.rs"),
    devastation: (10, "modules/devastation.rs"),
    elemental_ascendance: (220, "modules/elemental_ascendance.rs"),
    essence_burst: (140, "modules/essence_burst.rs"),
    evoker_tier: (160, "modules/evoker_tier.rs"),
    execution_counters: (320, "modules/execution_counters.rs"),
    feral_frenzy: (260, "modules/feral_frenzy.rs"),
    feral_growth: (1031, "modules/feral_growth.rs"),
    feral_lunar: (1030, "modules/feral_lunar.rs"),
    foreground_procs: (310, "modules/foreground_procs.rs"),
    frost: (80, "modules/frost.rs"),
    germination: (200, "modules/germination.rs"),
    giantkiller: (170, "modules/giantkiller.rs"),
    havoc: (150, "modules/havoc.rs"),
    impact_procs: (330, "modules/impact_procs.rs"),
    lava_surge: (300, "modules/lava_surge.rs"),
    marksmanship: (380, "modules/marksmanship.rs"),
    marksmanship_explosive: (180, "modules/marksmanship_explosive.rs"),
    marksmanship_salvo: (2540, "modules/marksmanship_salvo.rs"),
    marksmanship_sentinel: (390, "modules/marksmanship_sentinel.rs"),
    melee_reset: (340, "modules/melee_reset.rs"),
    paid_cooldowns: (130, "modules/paid_cooldowns.rs"),
    periodic_counters: (270, "modules/periodic_counters.rs"),
    resource_values: (350, "modules/resource_values.rs"),
    ret_purpose: (210, "modules/ret_purpose.rs"),
    ret_resonance: (190, "modules/ret_resonance.rs"),
    ret_toll: (280, "modules/ret_toll.rs"),
    retribution: (60, "modules/retribution.rs"),
    rogue: (70, "modules/rogue.rs"),
    secondary_copies: (360, "modules/secondary_copies.rs"),
    shadow_priest: (290, "modules/shadow_priest.rs"),
    snakeskin_quiver: (230, "modules/snakeskin_quiver.rs"),
    spellslinger_hits: (401, "modules/spellslinger_hits.rs"),
    touch_of_death: (240, "modules/touch_of_death.rs"),
    voltaic_blaze: (250, "modules/voltaic_blaze.rs"),
    warrior: (40, "modules/warrior.rs"),
    windwalker_momentum: (50, "modules/windwalker_momentum.rs"),
    windwalker_zenith: (30, "modules/windwalker_zenith.rs"),
}
