//! Source-qualified relationship data; execution remains in combat.
use shared::components::PowerType;

#[derive(Clone, Copy)]
pub struct ChannelMomentum {
    pub channel: u32,
    pub damage: u32,
    pub stacks: u32,
    pub speed: u32,
    pub haste_effect: u8,
}

#[derive(Clone, Copy)]
pub enum Relationship {
    OwnedPetWrathAfterCast {
        parent: u32,
    },
    ReduceCooldownAfterCast {
        parent: u32,
        cooldown: u32,
        milliseconds_effect: u8,
    },
    OwnedPetSpellAfterCast {
        parent: u32,
        payload: u32,
    },
    TriggerPrimaryAfterCast {
        parent: u32,
        payload: u32,
    },
    SpenderPeriodicExtension {
        resource: PowerType,
        payload: u32,
        impact_effect: u8,
        area_parent: u32,
        area_effect: u8,
    },
    PeriodicSplash {
        parent: u32,
        payload: u32,
        magnitude_spell: u32,
        percent_effect: u8,
        cap_effect: u8,
        radius_slot: u8,
    },
    ApplySelfAuraAfterCast {
        parent: u32,
        payload: u32,
    },
    ConsumeSelfAuraAfterCast {
        parent: u32,
        payload: u32,
    },
    SpendSelfAuraStackAfterCast {
        parent: u32,
        payload: u32,
    },
    ProcReplacement {
        spell: u32,
        aura: u32,
        triggers: &'static [(u32, u8)],
        mean_chance: f64,
    },
    ChannelMomentum(ChannelMomentum),
    ReapplicationExtend {
        payload: u32,
    },
    LinkedAura {
        parent: u32,
        payload: u32,
    },
    CounterEchoAfterCast {
        parent: u32,
        counter: u32,
        payload: u32,
        effectiveness_effect: u8,
    },
    CastEcho {
        parent: u32,
        payload: u32,
        required_auras: &'static [u32],
        effectiveness_effect: u8,
    },
    SecondaryEffect {
        parent: u32,
        effect: u8,
        cap_effect: u8,
    },
    HitPayload {
        parent: u32,
        payload: u32,
        self_target: bool,
        primary_only: bool,
    },
    CounterAfterCast {
        parent: u32,
        counter: u32,
        count_effect: u8,
    },
    SpenderCooldownRefund {
        resource: PowerType,
        parents: &'static [u32],
        cooldowns: &'static [u32],
        tenths_effect: u8,
        bonus_states: &'static [u32],
        bonus_spell: u32,
        bonus_effect: u8,
    },
    SpenderAuraExtension {
        resource: PowerType,
        parents: &'static [u32],
        payload: u32,
        seconds_effect: u8,
    },
    SpenderHit {
        resource: PowerType,
        counter: u32,
        payload: u32,
    },
}

#[derive(Clone, Copy)]
pub struct Policy {
    pub name: &'static str,
    pub spec: u32,
    pub talent: u32,
    pub relationship: Relationship,
}
