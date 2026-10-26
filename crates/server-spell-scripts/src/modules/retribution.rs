//! Explicit missing-script relationships; native records retain payload ownership.
use super::combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};
use shared::components::PowerType;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Policies(POLICIES)];

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_paladin.hpp1492–1496/1581–1594: every spender
    // impact extends owned Dawnlight; primary Storm adds its multi-DoT bonus.
    Policy {
        name: "SIMC_RET_ENDLESS_GLEAM",
        spec: 70,
        talent: 1263787,
        relationship: Relationship::SpenderPeriodicExtension {
            resource: PowerType::HolyPower,
            payload: 431380,
            impact_effect: 1,
            area_parent: 53385,
            area_effect: 2,
        },
    },
    // SimC76f8118 sc_paladin.cpp2990–2998/3048–3053: resolved tick
    // times native percent, secondaries only; source also uses effect1 for cap.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_PERIODIC_SPLASH",
        spec: 70,
        talent: 431377,
        relationship: Relationship::PeriodicSplash {
            parent: 431380,
            payload: 431399,
            magnitude_spell: 431581,
            percent_effect: 0,
            cap_effect: 0,
            radius_slot: 1,
        },
    },
    // SimC76f8118 sc_paladin.cpp3016–3026: DOT_EXTEND;
    // source note: "per bolas test Aug 21 2024". Native duration/phase stay owned.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_REAPPLICATION_EXTEND",
        spec: 70,
        talent: 431377,
        relationship: Relationship::ReapplicationExtend { payload: 431380 },
    },
    // SimC76f8118 sc_paladin.cpp314–315: Hammer mode follows wings.
    Policy {
        name: "SIMC_RET_WRATH_HAMMER_MODE",
        spec: 70,
        talent: 1241288,
        relationship: Relationship::LinkedAura {
            parent: 31884,
            payload: 1277026,
        },
    },
    Policy {
        name: "SIMC_RET_BACKGROUND_WRATH_HAMMER_MODE",
        spec: 70,
        talent: 1241288,
        relationship: Relationship::LinkedAura {
            parent: 454351,
            payload: 1277026,
        },
    },
    // SimC76f8118 sc_paladin.cpp1571–1575,1633–1642,1738–1743:
    // only foreground Judgment/HoW spends one native Resonance stack; its
    // background echo must not classify/consume another stack.
    Policy {
        name: "SIMC_RET_RESONANCE_JUDGMENT",
        spec: 70,
        talent: 384027,
        relationship: Relationship::CounterEchoAfterCast {
            parent: 20271,
            counter: 1266308,
            payload: 20271,
            effectiveness_effect: 1,
        },
    },
    Policy {
        name: "SIMC_RET_RESONANCE_HAMMER",
        spec: 70,
        talent: 384027,
        relationship: Relationship::CounterEchoAfterCast {
            parent: 24275,
            counter: 1266308,
            payload: 24275,
            effectiveness_effect: 1,
        },
    },
    // SimC76f8118 sc_paladin.cpp1745–1748; Ret1126–1127:
    // HoW during wings mirrors BoJ at native talent effectiveness and ICD.
    Policy {
        name: "SIMC_RET_WALK_INTO_LIGHT",
        spec: 70,
        talent: 1263782,
        relationship: Relationship::CastEcho {
            parent: 24275,
            payload: 184575,
            required_auras: &[31884, 454351],
            effectiveness_effect: 2,
        },
    },
    Policy {
        name: "SIMC_RET_TOLL_WALK_INTO_LIGHT",
        spec: 70,
        talent: 1263782,
        relationship: Relationship::CastEcho {
            parent: 1279408,
            payload: 184575,
            required_auras: &[31884, 454351],
            effectiveness_effect: 2,
        },
    },
    // SimC76f8118 sc_paladin.cpp1481–1486,1550–1564,1689–1698:
    // primary uses effect1; Blessed Champion adds only secondary recipients.
    Policy {
        name: "SIMC_RET_JUDGMENT_SECONDARIES",
        spec: 70,
        talent: 403010,
        relationship: Relationship::SecondaryEffect {
            parent: 20271,
            effect: 2,
            cap_effect: 0,
        },
    },
    Policy {
        name: "SIMC_RET_HAMMER_SECONDARIES",
        spec: 70,
        talent: 403010,
        relationship: Relationship::SecondaryEffect {
            parent: 24275,
            effect: 2,
            cap_effect: 3,
        },
    },
    // SimC76f8118 sc_paladin.cpp1502–1521,1580–1590: Toll Judgment earns HP.
    Policy {
        name: "SIMC_RET_TOLL_JUDGMENT_HOLY_POWER",
        spec: 70,
        talent: 315867,
        relationship: Relationship::HitPayload {
            parent: 406957,
            payload: 220637,
            self_target: true,
            primary_only: true,
        },
    },
    // SimC76f8118 sc_paladin.cpp888–916: each successful primary strike
    // grants the native406834 effect0, independently of the old stack counter.
    Policy {
        name: "SIMC_RET_CRUSADING_STRIKE_HOLY_POWER",
        spec: 70,
        talent: 404542,
        relationship: Relationship::HitPayload {
            parent: 408385,
            payload: 406834,
            self_target: true,
            primary_only: true,
        },
    },
    // SimC76f8118 sc_paladin_retribution.cpp432–436: BoJ impact applies
    // Expurgation. Native404358 carries the secondary BoJ target damage.
    Policy {
        name: "SIMC_RET_BLADE_EXPURGATION",
        spec: 70,
        talent: 383344,
        relationship: Relationship::HitPayload {
            parent: 184575,
            payload: 383346,
            self_target: false,
            primary_only: false,
        },
    },
    Policy {
        name: "SIMC_RET_BLADE_SECONDARY_EXPURGATION",
        spec: 70,
        talent: 383344,
        relationship: Relationship::HitPayload {
            parent: 404358,
            payload: 383346,
            self_target: false,
            primary_only: false,
        },
    },
    // sc_paladin_retribution.cpp846–876; native raw2 + specialization1 wins.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_WAKE_PRODUCER",
        spec: 70,
        talent: 431377,
        relationship: Relationship::CounterAfterCast {
            parent: 255937,
            counter: 431522,
            count_effect: 0,
        },
    },
    // sc_paladin.hpp1512–1535: successful foreground first hit, one use.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_SPENDER_CONSUMER",
        spec: 70,
        talent: 431377,
        relationship: Relationship::SpenderHit {
            resource: PowerType::HolyPower,
            counter: 431522,
            payload: 431380,
        },
    },
];
