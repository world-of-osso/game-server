//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::ChannelMomentum;
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

const POLICIES: &[Policy] = &[
    // Momentum Boost tooltip: https://warcraft.wiki.gg/wiki/Momentum_Boost
    // SimC76f8118 sc_monk.cpp1699–1704/1730–1734/1839–1841: haste damage,
    // FoF-only stacks, reset and native60%/8s speed on end (also dot.cpp75–81).
    Policy {
        name: "SIMC_WW_MOMENTUM_BOOST",
        spec: 269,
        talent: 451294,
        relationship: Relationship::ChannelMomentum(ChannelMomentum {
            channel: 113656,
            damage: 117418,
            stacks: 451297,
            speed: 451298,
            haste_effect: 0,
        }),
    },
];
