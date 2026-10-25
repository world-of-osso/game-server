//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_warrior.cpp3565–3581: Heroic Strike requires and consumes
    // the actual Master of Warfare proc; its payload and costs stay native.
    Policy {
        name: "SIMC_ARMS_MASTER_OF_WARFARE_REPLACEMENT",
        spec: 71,
        talent: 1269314,
        relationship: Relationship::ProcReplacement {
            spell: 1269383,
            aura: 1269391,
            // Parent hits are foreground; Execute's native costed damage child is depth1.
            triggers: &[(12294, 0), (7384, 0), (1464, 0), (260798, 1)],
            // sc_warrior.cpp1666: explicitly15%, not the native101 script sentinel.
            mean_chance: 0.15,
        },
    },
    // SimC76f8118 sc_warrior.cpp3685–3686: Cleave applies the selected Rend
    // periodic payload on each landed target; native388539 owns all values.
    Policy {
        name: "SIMC_ARMS_CLEAVE_REND",
        spec: 71,
        talent: 772,
        relationship: Relationship::HitPayload {
            parent: 845,
            payload: 388539,
            self_target: false,
            primary_only: false,
        },
    },
    // SimC76f8118 sc_warrior.cpp4452–4456: only Fury Execute main-hand
    // applies Deep Wounds. Native262115 retains its coefficient and rolling pool.
    Policy {
        name: "SIMC_FURY_EXECUTE_DEEP_WOUNDS",
        spec: 72,
        talent: 1261060,
        relationship: Relationship::HitPayload {
            parent: 280849,
            payload: 262115,
            self_target: false,
            primary_only: false,
        },
    },
];
