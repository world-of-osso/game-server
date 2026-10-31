//! Downstream script dispatch. The ordered module slices are supplied by the host.
//!
//! Lookup order is part of the combat contract:
//! - `NativeAfterCast`: first matching module wins, before the native spell match.
//! - `AuraRollAfterCast`: all matching modules run after native, policies,
//!   spellslinger and batch9, and before resource-value hooks.
//! - `OnCast`: all modules run before combo tracking and elemental launch;
//!   Havoc transformation follows the earlier registered cast-grant modules.
//! - `AfterPayment`: all modules run after successful payment and before gargoyles.
//! - `AfterResourceValues`: all matching modules run after resource-value hooks.
//! - `TargetPlan`: first matching module wins before combat's native target plans.
//! - `PeriodicPrimaryDamage`: all modules receive the landed periodic amount and actual crit result.
//! - `TargetDamageMultiplier`: factors run at the Giantkiller damage boundary.
//! - `PeriodicTickRate`: factors divide each periodic interval when it is scheduled.
//! - `AfterSpenderStacks`: after foreground Crusade stacks and the corresponding
//!   background Storm callback; before Empyrean Legacy or echo policy dispatch.
//! - `AuraRedirect`: first redirect consumes application before native aura dispatch.
//! - `EffectHit`, `Hit`, `AreaFilter`, `CriticalChance`: first matching callback,
//!   before the retained native lookup. Target handling uses the same registry copy.
//! - `ContextHit` and `DamageAmount`: first match after generic policy lookup.
//! - `CheckTarget`: before native target checks; first error stops admission.
//! - `AutoRepeat`: after the actual Auto Shot launch.
//! - `EarlyAuraRegistration`: startup aura scripts, before native registration.
//! - `ConsumedEffectValues`: classifies script-read effect77 data, without dispatch.
//! - `LaunchDelay`: first explicit override, otherwise retain the native delay.
//! - `DotMasteryMultiplier`: factors at the original Shadow Weaving damage boundary.
//! - `CastTimeProcs` and `PreserveUnadmittedProcs`: admission markers and charged-mod
//!   preservation; the native class mask and instance IDs still own consumption.
//! - `PeriodicDamageFinished`: after actual periodic damage, before Ret periodic hooks.
//! - `AfterForegroundSpecial`: special-execution producers then execution counters.
//! - `AfterCounterLaunch`: after consumed elemental/pet launch state, before Unbroken Rhythm.
//! - `ImpactProcDamage`: after generic damage policies, before Ret damage policies.
//! - `AfterWhiteHit`: after Flurry; `ReplacementDirectDamage`: before generic damage policies.
//! - `OwnsPolicyProc`: native proc ownership classification, not a proc attempt.
//! - `ResourceValueAfterCast`: after aura rolls, before AfterResourceValues.
//! - `TriggeredResourceProducer`: before triggered relationship producers.
//! - `EffectiveResourceValue`: effective spend value only; actual payment is unchanged.
//! - `ConsumeResourceValueSources`: after successful foreground base execution.
//! - `SpellResidual`: direct, unresolved source damage before direct-damage callbacks.
//! - `MeleeResidual`: only after a landed white-hit outcome.
//! - `LateAuraRegistration`: after retained native aura registrations.
//! - `SuccessfulCastRelationships`: after foreground special/resource consumers.
//! - `AfterAbsorbConsumed`: after a shield actually absorbs damage.
//! - `AvoidanceProcHitMask`: sourced avoidance outcomes for a scripted proc.
//! - `GuaranteesCrit`, `IgnoresCooldown`, `SnapshotsModifier`, `ArmorIgnoredSpells`:
//!   pure native-readiness, launch snapshot and damage classification predicates.
//! - `AllowCastDuringChannel`: source-qualified native cast admission during an active channel.
//! - `PersistentCastMultiplier`: spell IDs whose existing cast factor is stored as periodic metadata.
//! - `ResolvedDamage`: after actual direct/periodic damage, before Explosive tick callbacks.
//! - `ResolvedHeal`: after health and combat log updates, with raw and effective healing.
//! - `AfterDirectDamageBeforeItems`: after Ret direct damage, before item scripts.
//! - `LandedDamagePolicy`: actual white/spell damage at the original Havoc policy boundary.
//! - `AbilityUseAvailable`: earned per-ability use state, not ordinary cast admission.
//! - `ChannelPeriodic`: first scripted periodic-trigger callback, before native trigger handling.
//! - `BeforeNativeOnCast`: admitted cast data before ordinary OnCast and payment.
//! - `PrepareStoredDamage`: after fixed payload overrides, before triggered admission.
//! - `EnemyContextHit`: damage-only hit callbacks, excluding caster/aura recipients.
//! - `AfterPolicyDirectDamage` and `AfterPolicyPeriodicDamage`: original Ret damage slots.
//! - `TargetContextDamageMultiplier`: after Giantkiller, before Shadow Weaving.
//! - `AfterNativeOnCast`: after native judgment energize and other OnCast work.
//! - `AfterModifierConsumption`: before foreground-spender tier callbacks.
//! - `AfterForegroundSpender`: before Crusade stacks and spender-proc generation.
//! - `AfterSpenderProc`: after spender-proc generation, before deferred background execution.
//! - `AfterElementalLaunch`: after Elemental consumption, before pet-set launch state.
//! - `BeforeDirectDamagePolicies`: before Spellslinger and other direct-damage policies.
//! - `AfterPeriodicDamagePolicies`: after generic periodic relationships and Spellslinger.
//! - `EmpowerCompletion`: first completion handler, before channel end and finish GCD.
//! - `EmpowerDamageBonus`: native converted-tick damage added before normal multipliers.
//! - `EmpowerReleasePreview`: first matching native release metadata for APL/fixtures.
//! - `CaptureConsumedDamageModifier`: after pet-set capture; first consuming source.
//! - `FinishConsumedDamageModifier`: before Frost/pet-set launch completion.
//! - `BeforeJudgmentOnCast`: after combo tracking, before native judgment energize.
//! - `BeforeLateAuraRegistration`: immediately before late relationship registration.
//! - `MidAuraRegistration`: native proc rows after Dance of Chi-Ji, before Cenarion Ward.
//! - `HealingMultiplier`: direct/periodic healing at the original persistent-heal slot.
//! - `SnapshotHealingAura`: after periodic spell-mod capture, before other aura amount scripts.
//! - `FireAuraRegistration`: after native Ret Sentence, before Fel Devastation.
//! - `BeforeProcLaunch` and `LaunchDamageModifier`: admitted launch snapshots before damage.
//! - `HotStreakProducers`: shared eligible flight/damage classification table.
//! - `AfterFireDirectHit`: positive direct hits after generic direct damage callbacks.
//! - `RecordFireOutcome`, `AfterFireOutcomeHit`: eligible hit outcomes, then Meteorite callbacks.
//! - `CastHitProcChance` and `RollCastHitProc`: chance snapshot before consumption, roll after cast.
//! - `AfterQueuedTriggers`: fixed update after queued hits, before power/rune sync.
//! - `AfterSpellPayment`: after generic payment callbacks, before gargoyle empowerment.
//! - `BeforeDamageSnapshot`: before consumed-modifier capture, preserving pet-set multiplier order.
//! - `BeforeExecutionCounterLaunch`: after Frost launch completion, before execution counters.
//! - `PolicyTailCast`, `PolicyTailHit`: first additional relationship handler after generic
//!   policy traversal, preserving the original first-hit/target guard and cast ordering.
//! - `ProcCharges`: transform the authored charge count at native aura creation.
//! - `ExtraComboPoints`: additional authored points at the positive root self-energize slot.
//! - `NamedHealingMultiplier`: factors before the persistent-healing multiplier.
//! - `EffectEnabled`: additive vetoes after other native effect filters, before guardians.
//! - `WeaponAuraRegistration`: after Rogue proc registration, before Spirit Wolf.
//! - `CheckNativeSpell`: at the original additional spell-admission check.
//! - `AfterResolvedHits`: after actual cast hits, before resolving-frame teardown.
//! - `SnapshotNativeAuraAmount`: at the original Rod aura-amount snapshot position.
//! - `AfterCastUpdate`: original delayed-weapon scheduler, ordered after advance_casts.
//! - `ConsumeCastModifiers`: successful foreground consumption at the original MW slot.
//! - `AfterWeaponLaunch`: tier then linked weapon payloads after Glory of the Dawn.
//! - `AfterPolicyCast`: first additional cast handler after generic relationship handlers.
//! - `BeforeMeleeComboDamage`: actual direct damage before melee/combo feedback.
//! - `BeforePeriodicItemDamage`: periodic relationship completion before item callbacks.
//! - `ConjureCount`: explicit source-qualified extra-projectile requests, without a second cast.
//! - `AfterAuraApplication`: after successful aura storage and native apply/refresh callbacks.
//! - `SummonInitialized`: after guardian initialization, receiving the actual spawned actor.
//! - `SummonExpired`: at natural expiry, before the actual actor is despawned.
//! - `GuardianActionEligible`: additive vetoes on a ready bound guardian action, before its cast.
//! New stages own distinct callback/lookup aliases, even when signatures coincide.
//! Within a stage, traverse module slices and entries from first to last. Never sort
//! or deduplicate them. Adding a policy to an existing module changes only that module.

mod policy;
pub use policy::{ChannelMomentum, Policy, Relationship};

use bevy::prelude::*;

use super::{CostPool, TargetPlan, effects::EffectCtx, scripts::AfterCastHook, targets::Relation};
use crate::spell_info::SpellInfo;

pub type TargetPlanHook = fn(&mut World, Entity, &SpellInfo, Option<Entity>) -> TargetPlan;
pub type TargetPlanLookup = fn(u32) -> Option<(Relation, TargetPlanHook)>;
pub type PeriodicPrimaryDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, f32, bool);
pub type SpellResidualHook = fn(&mut World, &EffectCtx<'_>, Entity, f32);
pub type TargetDamageMultiplierHook = fn(&World, Entity, Entity, &SpellInfo) -> f32;
/// Caster, aura target and periodic spell; the result divides the next interval.
pub type PeriodicTickRateHook = fn(&World, Entity, Entity, &SpellInfo) -> f32;
pub type AuraApplicationHook = fn(&mut World, Entity, &crate::auras::AuraInstance);
pub type SummonInitializedHook = fn(&mut World, Entity);
/// Actual CombatStatus transitions, after native reference state is updated.
pub type CombatStatusChangedHook = fn(&mut World, Entity, bool);
pub type SummonExpiredHook = fn(&mut World, Entity);
pub type SummonRemovedHook = fn(&mut World, Entity);
/// Actual guardian actor, its bound action and selected enemy.
pub type GuardianActionEligibleHook = fn(&World, Entity, u32, Entity) -> bool;
pub type AuraRedirectHook = fn(&mut World, Entity, Entity, u32, u8) -> bool;

pub type AfterCastLookup = fn(u32) -> Option<AfterCastHook>;
pub type OnCastHook = fn(&mut World, Entity, u32, u8);
/// Actor-conditioned channel behavior supplied by a source-qualified script.
pub type ScriptedChannelHook = fn(&World, Entity, &SpellInfo) -> bool;
pub type AfterPaymentHook = fn(&mut World, Entity, &[(CostPool, f32)]);
pub type PowerCostHook = fn(&World, Entity, &SpellInfo, CostPool, f32) -> f32;
pub type HitHook = fn(&mut World, Entity, Entity, u8);
pub type EffectHitLookup = fn(u32, u8) -> Option<HitHook>;
pub type EffectHandlerLookup = fn(u32, u8) -> Option<super::effects::EffectHandler>;
pub type TargetFilterHook = fn(&World, Entity, &SpellInfo, u8, Option<Entity>, &mut Vec<Entity>);
pub type HitLookup = fn(u32) -> Option<HitHook>;
pub type ContextHitHook = fn(&mut World, &EffectCtx<'_>, Entity);
pub type ContextHitLookup = fn(u32) -> Option<ContextHitHook>;
pub type AreaTargetFilter = fn(&World, &SpellInfo, Option<Entity>, &mut Vec<Entity>);
pub type AreaFilterLookup = fn(u32, u8) -> Option<AreaTargetFilter>;
pub type CheckTargetHook = fn(&World, Entity, u32, Entity) -> Result<(), super::CastFailure>;
pub type ActualDamageMultiplierHook = fn(&mut World, &EffectCtx<'_>, Entity) -> f32;
pub type DamageAmountHook = fn(&World, Entity, Entity, f32) -> f32;
pub type DamageAmountLookup = fn(u32) -> Option<DamageAmountHook>;
pub type AutoRepeatHook = fn(&mut World, Entity, Entity);
pub type CriticalChanceHook = fn(&World, Entity, Entity, u32) -> Option<f32>;
pub type ActualCriticalChanceBonusHook = fn(&World, Entity, Entity, &SpellInfo, f32) -> f32;
pub type ActualCriticalDamageMultiplierHook = fn(&World, Entity, &SpellInfo) -> f32;
pub type AuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);
pub type LaunchDelayLookup = fn(u32) -> Option<f32>;
pub type DotMasteryMultiplierHook = fn(&World, Entity, Entity, u32) -> f32;
pub type PeriodicDamageFinishedHook = fn(&mut World, &EffectCtx<'_>);
pub type ActionHook = fn(&mut World, Entity, u32);
pub type ComboPointGainHook = fn(&mut World, Entity, u32, u8);
pub type WhiteHitHook = fn(&mut World, Entity, bool);
pub type WhiteHandHitHook = fn(&mut World, Entity, crate::melee::Hand, bool);
pub type WhiteImpactHook = fn(&mut World, Entity);
pub type DirectDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, bool);
pub type OwnsProcLookup = fn(u32) -> bool;
pub type EffectiveResourceValueHook =
    fn(&World, Entity, &SpellInfo, shared::components::PowerType, f32) -> f32;
pub type MeleeResidualHook = fn(&mut World, Entity, Entity, f32);
pub type MeleeRawResolvedHook = fn(&mut World, Entity, Entity, f32, bool);
pub type CastPredicate = fn(&World, Entity, &SpellInfo) -> bool;
pub type ResolvedDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, bool, f32);
pub type RawResolvedDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, bool, f32);
pub type ResolvedHealHook = fn(&mut World, &EffectCtx<'_>, Entity, f32, f32, bool, bool);
pub type LandedDamageHook = fn(&mut World, Entity, Entity, Option<u32>, f32, bool);
pub type AbilityUseLookup = fn(&World, Entity, u32) -> bool;
pub type ChannelPeriodicLookup = fn(u32, u8) -> Option<ContextHitHook>;
pub type BeforeNativeOnCastHook = fn(&mut World, Entity, &SpellInfo, u8);
pub type PrepareStoredDamageHook = fn(&mut SpellInfo);
pub type EnemyContextHitHook = fn(&mut World, &EffectCtx<'_>, Entity);
pub type EnemyContextHitLookup = fn(u32) -> Option<EnemyContextHitHook>;
pub type AfterPolicyDirectDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, bool);
pub type AfterPolicyPeriodicDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, bool);
pub type TargetContextDamageMultiplierHook = fn(&World, &EffectCtx<'_>, Entity, bool) -> f32;
pub type AfterNativeOnCastHook = fn(&mut World, Entity, u32, u8);
pub type AfterComboGainHook = fn(&mut World, &EffectCtx<'_>, Entity, f32);
pub type AfterComboOverflowHook = fn(&mut World, Entity, f32);
pub type AfterModifierConsumptionHook = fn(&mut World, Entity, &SpellInfo);
pub type AfterForegroundSpenderHook = fn(&mut World, Entity, &SpellInfo, Option<Entity>);
pub type AfterSpenderProcHook = fn(&mut World, Entity, &SpellInfo, Option<Entity>);
pub type AfterElementalLaunchHook = fn(&mut World, Entity, &SpellInfo, bool);
pub type AreaTriggerExpiredHook = fn(&mut World, Entity, u32, Vec3);
pub type BeforeDirectDamagePoliciesHook = fn(&mut World, &EffectCtx<'_>, Entity, bool);
pub type AfterPeriodicDamagePoliciesHook = fn(&mut World, &EffectCtx<'_>, Entity);
pub type EmpowerCompleteHook =
    fn(&mut World, Entity, &SpellInfo, Option<Entity>) -> Result<(), super::CastFailure>;
pub type EmpowerCompleteLookup = fn(u32) -> Option<EmpowerCompleteHook>;
pub type EmpowerDamageBonusHook = fn(&World, Entity, &SpellInfo) -> f32;
pub type EmpowerReleasePreviewHook = fn(&World, Entity, u32, u8) -> Option<SpellInfo>;
pub type CaptureConsumedDamageModifierHook =
    fn(&World, Entity, &mut SpellInfo, bool) -> Option<u32>;
pub type FinishConsumedDamageModifierHook =
    fn(&mut World, Entity, &SpellInfo, Option<u32>, u8, bool);
pub type BeforeJudgmentOnCastHook = fn(&mut World, Entity, u32, u8);
pub type BeforeLateAuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);
pub type BeforeDamageSnapshotHook = fn(&World, Entity, &mut SpellInfo);
pub type BeforeExecutionCounterLaunchHook = fn(&mut World, Entity, &SpellInfo);
pub type PolicyTailCastHook = fn(&mut World, Entity, &SpellInfo);
pub type PolicyTailCastLookup = fn(u32) -> Option<PolicyTailCastHook>;
pub type PolicyTailHitHook = fn(&mut World, &EffectCtx<'_>);
pub type PolicyTailHitLookup = fn(u32) -> Option<PolicyTailHitHook>;
pub type AfterQueuedTriggersHook = fn(&mut World);
pub type AfterSpellPaymentHook = fn(&mut World, Entity, &SpellInfo, &[(CostPool, f32)]);
pub type AfterPeriodicEnergizeHook = fn(&mut World, Entity, u32, u8);
pub type FireAuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);
pub type BeforeProcLaunchHook = fn(&mut World, Entity, &SpellInfo);
pub type AfterFireDirectHitHook = fn(&mut World, &EffectCtx<'_>, Entity, bool);
pub type AfterFireOutcomeHitHook = fn(&mut World, &EffectCtx<'_>, Entity);
pub type RecordFireOutcomeHook = fn(&mut World, Entity, &SpellInfo, bool, bool);
pub type CastHitProcChanceHook = fn(&World, Entity, &SpellInfo, Option<Entity>) -> Option<f32>;
pub type RollCastHitProcHook = fn(&mut World, Entity, f32);
pub type LaunchDamageModifierHook =
    fn(&World, Entity, &SpellInfo) -> Option<crate::spell_info::ChargedSpellModifier>;
pub type MidAuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);
pub type HealingMultiplierHook = fn(&World, Entity, &SpellInfo, bool) -> f32;
pub type LeechHealOverrideHook = fn(&World, &EffectCtx<'_>, f32) -> Option<f32>;
pub type SnapshotHealingAuraHook =
    fn(&World, Entity, &SpellInfo, &crate::spell_info::SpellEffect, f32) -> f32;

pub type ProcChargesHook = fn(&World, Entity, &SpellInfo, f32) -> f32;
pub type ExtraComboPointsHook = fn(&World, Entity, &SpellInfo) -> f32;
pub type NamedHealingMultiplierHook = fn(&World, Entity, &SpellInfo) -> f32;
pub type EffectEnabledHook = fn(&World, Entity, u32, u8) -> bool;

pub type WeaponAuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);
pub type CheckNativeSpellHook = fn(&World, Entity, &SpellInfo) -> Result<(), super::CastFailure>;
pub type AfterResolvedHitsHook = fn(&mut World, &EffectCtx<'_>);
pub type SnapshotNativeAuraAmountHook = fn(&World, &SpellInfo, u8, f32) -> f32;
pub type AfterCastUpdateHook = fn(&mut World);
pub type ConsumeCastModifiersHook = fn(&mut World, Entity, &SpellInfo);
pub type AfterWeaponLaunchHook = fn(&mut World, Entity, &SpellInfo, u8);

pub type PolicyContextRequiredHook = fn(&World, Entity, &SpellInfo) -> bool;
pub type PreparePolicySpellHook = fn(&World, Entity, &mut SpellInfo);
pub type BeforePolicyPrimaryHitHook = fn(&mut World, &EffectCtx<'_>, Entity);
pub type RogueAuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);
pub type BeforeEnemyContextHitHook = fn(&mut World, &EffectCtx<'_>, Entity);
pub type BeforeEnemyContextHitLookup = fn(u32) -> Option<BeforeEnemyContextHitHook>;
pub type HealthChangeObservedHook = fn(&mut World, &shared::protocol::CombatLogEvent);
pub type SpellHealthImpactHook = fn(&mut World, &EffectCtx<'_>, Entity, f32, bool);
pub type AuraEventHook = fn(&mut World, &crate::auras::procs::AuraEvent);
pub type RawHealHook = fn(&mut World, &EffectCtx<'_>, f32);
pub type ExpelHarmHealHook = fn(&mut World, Entity) -> f32;
pub type IncomingDamageBeforeMitigationHook = fn(&mut World, Entity, Option<u32>, f32);
pub type HealReceivedHook = fn(&mut World, Entity, u32, f32, f32, bool);
pub type SupportAuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);
pub type NamedAuraFields = &'static [(u32, u8)];
pub type SnapshotLaunchAurasHook = fn(&World, Entity, &mut SpellInfo);
pub type TriggeredChildDamageMultiplierHook = fn(&World, Entity, &SpellInfo, u32) -> Option<f32>;
pub type ProcAfterCastLookup = fn(u32) -> Option<AfterCastHook>;
pub type SupplyAfterCastLookup = fn(u32) -> Option<AfterCastHook>;
pub type ResolvePendingScriptEventsHook = fn(&mut World);
pub type AfterLaunchProcSupplyHook = fn(&mut World, Entity, &SpellInfo);

pub type ConjureFromSpellHook = fn(&mut World, Entity, &SpellInfo, Entity, u32);
pub type BeforeScriptOnCastHook = fn(&mut World, Entity, u32, u8);
pub type AfterWeaponDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, f32);
pub type NativeWeaponMultiplierHook = fn(&World, Entity, &SpellInfo, u32) -> Option<f32>;
pub type AfterNativeLaunchHook = fn(&mut World, Entity, &SpellInfo);
pub type AfterNativeCastUpdateHook = fn(&mut World);
pub type AfterChainResolvedHook = fn(&mut World, &EffectCtx<'_>);
pub type BeforePendingScriptEventsHook = fn(&mut World);
pub type ConjureCountHook = fn(&mut World, Entity, Entity, u32);
pub type AfterPolicyCastHook = fn(&mut World, Entity, &SpellInfo);
pub type AfterPolicyCastLookup = fn(u32) -> Option<AfterPolicyCastHook>;
pub type BeforeMeleeComboDamageHook = fn(&mut World, &EffectCtx<'_>);
pub type BeforePeriodicItemDamageHook = fn(&mut World, &EffectCtx<'_>, Entity);

/// Maximum hostile secondary-target distance from the actor and its primary target.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeclaredReach {
    pub caster_radius: f32,
    pub target_radius: f32,
}

/// Effects restricted to already-afflicted enemies cannot acquire an idle pack.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ScriptReach {
    #[default]
    None,
    Area(DeclaredReach),
    EngagedOnly,
    /// A bounded selection that can only reach enemies already in combat.
    EngagedArea(DeclaredReach),
}

impl ScriptReach {
    pub fn caster(radius: f32) -> Self {
        Self::Area(DeclaredReach {
            caster_radius: radius,
            target_radius: 0.0,
        })
    }

    pub fn target(radius: f32) -> Self {
        Self::Area(DeclaredReach {
            caster_radius: 0.0,
            target_radius: radius,
        })
    }
}

/// A selected passive may be represented by its spellbook entry or its active aura.
pub fn reach_source_selected(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<crate::class_progression::KnownSpells>(caster)
        .is_some_and(|known| known.contains(spell))
        || world
            .get::<crate::class_progression::TraitLoadout>(caster)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == spell)
            })
        || crate::auras::has_aura(world, caster, spell)
}

pub type DeclaredReachHook = fn(&World, Entity, &SpellInfo) -> ScriptReach;

#[derive(Clone, Copy)]
pub enum SpellScriptHook {
    DeclaredReach(DeclaredReachHook),
    /// Autonomous actions used by this unit outside its ordinary spellbook.
    AdditionalReachActions(fn(&World, Entity) -> Vec<u32>),
    Policies(&'static [Policy]),
    NativeAfterCast(AfterCastLookup),
    ScriptedChannel(ScriptedChannelHook),
    AuraRollAfterCast(AfterCastLookup),
    OnCast(OnCastHook),
    AfterPayment(AfterPaymentHook),
    PowerCost(PowerCostHook),
    AfterResourceValues(AfterCastLookup),
    TargetPlan(TargetPlanLookup),
    PeriodicPrimaryDamage(PeriodicPrimaryDamageHook),
    CounterOnlyDotStacks(&'static [u32]),
    /// Source-owned refreshes retain the already scheduled periodic deadline.
    PreservePeriodicPhase(&'static [u32]),
    TargetDamageMultiplier(TargetDamageMultiplierHook),
    PeriodicTickRate(PeriodicTickRateHook),
    AfterSpenderStacks(AfterCastHook),
    AuraRedirect(AuraRedirectHook),
    /// All successful native aura applications, including refresh/replacement.
    AfterAuraApplication(AuraApplicationHook),
    SummonInitialized(SummonInitializedHook),
    CombatStatusChanged(CombatStatusChangedHook),
    SummonExpired(SummonExpiredHook),
    SummonRemoved(SummonRemovedHook),
    GuardianActionEligible(GuardianActionEligibleHook),
    EffectHit(EffectHitLookup),
    EffectHandler(EffectHandlerLookup),
    TargetFilter(TargetFilterHook),
    Hit(HitLookup),
    ContextHit(ContextHitLookup),
    AreaFilter(AreaFilterLookup),
    CheckTarget(CheckTargetHook),
    DamageAmount(DamageAmountLookup),
    ActualDamageMultiplier(ActualDamageMultiplierHook),
    ExpectedDamageMultiplier(TargetDamageMultiplierHook),
    AutoRepeat(AutoRepeatHook),
    CriticalChance(CriticalChanceHook),
    ActualCriticalChanceBonus(ActualCriticalChanceBonusHook),
    ActualCriticalDamageMultiplier(ActualCriticalDamageMultiplierHook),
    EarlyAuraRegistration(AuraRegistrationHook),
    ConsumedEffectValues(&'static [(u32, u8)]),
    /// Explicit source-script override of synchronous native stacking.
    AsynchronousAuras(&'static [u32]),
    /// Stored target uses do not multiply an incoming damage effect's magnitude.
    NonStackingIncomingAuras(&'static [u32]),
    /// Source scripts that consume the existing frozen effective-resource snapshot.
    ResourceContext(&'static [u32]),
    /// Payment-only consumers do not require the hostile primary among native self-aura targets.
    ResourceOnlyContext(&'static [u32]),
    LaunchDelay(LaunchDelayLookup),
    DotMasteryMultiplier(DotMasteryMultiplierHook),
    CastTimeProcs(&'static [u32]),
    PreserveUnadmittedProcs(&'static [u32]),
    PeriodicDamageFinished(PeriodicDamageFinishedHook),
    AfterForegroundSpecial(AfterCastHook),
    AfterCounterLaunch(ActionHook),
    ImpactProcDamage(ContextHitHook),
    ComboPointGain(ComboPointGainHook),
    AfterWhiteHit(WhiteHitHook),
    AfterWhiteHandHit(WhiteHandHitHook),
    AfterWhiteImpact(WhiteImpactHook),
    ReplacementDirectDamage(DirectDamageHook),
    OwnsPolicyProc(OwnsProcLookup),
    ResourceValueAfterCast(AfterCastLookup),
    TriggeredResourceProducer(AfterCastHook),
    EffectiveResourceValue(EffectiveResourceValueHook),
    ConsumeResourceValueSources(AfterCastHook),
    SpellResidual(SpellResidualHook),
    MeleeResidual(MeleeResidualHook),
    MeleeRawResolved(MeleeRawResolvedHook),
    LateAuraRegistration(AuraRegistrationHook),
    SuccessfulCastRelationships(AfterCastHook),
    AfterAbsorbConsumed(fn(&mut World, Entity, Entity, Option<Entity>, u32, f32)),
    AvoidanceProcHitMask(fn(u32) -> Option<u32>),
    GuaranteesCrit(CastPredicate),
    IgnoresCooldown(CastPredicate),
    AllowCastDuringChannel(CastPredicate),
    SnapshotsModifier(OwnsProcLookup),
    /// These spells retain their existing cast damage factor as stored periodic metadata.
    PersistentCastMultiplier(&'static [u32]),
    ArmorIgnoredSpells(&'static [u32]),
    ResolvedDamage(ResolvedDamageHook),
    RawResolvedDamage(RawResolvedDamageHook),
    ResolvedHeal(ResolvedHealHook),
    AfterDirectDamageBeforeItems(PeriodicDamageFinishedHook),
    LandedDamagePolicy(LandedDamageHook),
    AbilityUseAvailable(AbilityUseLookup),
    /// Sourced foreground readiness, checked before knowledge aliases and cast admission.
    ActionAvailable(AbilityUseLookup),
    ChannelPeriodic(ChannelPeriodicLookup),
    BeforeNativeOnCast(BeforeNativeOnCastHook),
    PrepareStoredDamage(PrepareStoredDamageHook),
    EnemyContextHit(EnemyContextHitLookup),
    AfterPolicyDirectDamage(AfterPolicyDirectDamageHook),
    AfterPolicyPeriodicDamage(AfterPolicyPeriodicDamageHook),
    TargetContextDamageMultiplier(TargetContextDamageMultiplierHook),
    AfterNativeOnCast(AfterNativeOnCastHook),
    AfterComboGain(AfterComboGainHook),
    AfterComboOverflow(AfterComboOverflowHook),
    AfterModifierConsumption(AfterModifierConsumptionHook),
    AfterForegroundSpender(AfterForegroundSpenderHook),
    AfterSpenderProc(AfterSpenderProcHook),
    AfterElementalLaunch(AfterElementalLaunchHook),
    AreaTriggerExpired(AreaTriggerExpiredHook),
    BeforeDirectDamagePolicies(BeforeDirectDamagePoliciesHook),
    AfterPeriodicDamagePolicies(AfterPeriodicDamagePoliciesHook),
    EmpowerCompletion(EmpowerCompleteLookup),
    EmpowerDamageBonus(EmpowerDamageBonusHook),
    EmpowerReleasePreview(EmpowerReleasePreviewHook),
    CaptureConsumedDamageModifier(CaptureConsumedDamageModifierHook),
    FinishConsumedDamageModifier(FinishConsumedDamageModifierHook),
    BeforeJudgmentOnCast(BeforeJudgmentOnCastHook),
    BeforeLateAuraRegistration(BeforeLateAuraRegistrationHook),
    BeforeDamageSnapshot(BeforeDamageSnapshotHook),
    BeforeExecutionCounterLaunch(BeforeExecutionCounterLaunchHook),
    PolicyTailCast(PolicyTailCastLookup),
    PolicyTailHit(PolicyTailHitLookup),
    AfterQueuedTriggers(AfterQueuedTriggersHook),
    AfterSpellPayment(AfterSpellPaymentHook),
    AfterPeriodicEnergize(AfterPeriodicEnergizeHook),
    FireAuraRegistration(FireAuraRegistrationHook),
    BeforeProcLaunch(BeforeProcLaunchHook),
    AfterFireDirectHit(AfterFireDirectHitHook),
    AfterFireOutcomeHit(AfterFireOutcomeHitHook),
    HotStreakProducers(&'static [u32]),
    RecordFireOutcome(RecordFireOutcomeHook),
    CastHitProcChance(CastHitProcChanceHook),
    RollCastHitProc(RollCastHitProcHook),
    LaunchDamageModifier(LaunchDamageModifierHook),
    MidAuraRegistration(MidAuraRegistrationHook),
    HealingMultiplier(HealingMultiplierHook),
    LeechHealOverride(LeechHealOverrideHook),
    SnapshotHealingAura(SnapshotHealingAuraHook),
    ProcCharges(ProcChargesHook),
    ExtraComboPoints(ExtraComboPointsHook),
    NamedHealingMultiplier(NamedHealingMultiplierHook),
    EffectEnabled(EffectEnabledHook),
    WeaponAuraRegistration(WeaponAuraRegistrationHook),
    CheckNativeSpell(CheckNativeSpellHook),
    AfterResolvedHits(AfterResolvedHitsHook),
    SnapshotNativeAuraAmount(SnapshotNativeAuraAmountHook),
    AfterCastUpdate(AfterCastUpdateHook),
    ConsumeCastModifiers(ConsumeCastModifiersHook),
    AfterWeaponLaunch(AfterWeaponLaunchHook),
    PolicyContextRequired(PolicyContextRequiredHook),
    PreparePolicySpell(PreparePolicySpellHook),
    BeforePolicyPrimaryHit(BeforePolicyPrimaryHitHook),
    RogueAuraRegistration(RogueAuraRegistrationHook),
    BeforeEnemyContextHit(BeforeEnemyContextHitLookup),
    HealthChangeObserved(HealthChangeObservedHook),
    SpellHealthImpact(SpellHealthImpactHook),
    AuraEvent(AuraEventHook),
    RawHeal(RawHealHook),
    ExpelHarmHeal(ExpelHarmHealHook),
    IncomingDamageBeforeMitigation(IncomingDamageBeforeMitigationHook),
    HealReceived(HealReceivedHook),
    SupportAuraRegistration(SupportAuraRegistrationHook),
    NamedAuraFields(NamedAuraFields),
    SnapshotLaunchAuras(SnapshotLaunchAurasHook),
    TriggeredChildDamageMultiplier(TriggeredChildDamageMultiplierHook),
    ProcAfterCast(ProcAfterCastLookup),
    SupplyAfterCast(SupplyAfterCastLookup),
    ResolvePendingScriptEvents(ResolvePendingScriptEventsHook),
    AfterLaunchProcSupply(AfterLaunchProcSupplyHook),
    ConjureCount(ConjureCountHook),
    ConjureFromSpell(ConjureFromSpellHook),
    BeforeScriptOnCast(BeforeScriptOnCastHook),
    AfterWeaponDamage(AfterWeaponDamageHook),
    NativeWeaponMultiplier(NativeWeaponMultiplierHook),
    AfterNativeLaunch(AfterNativeLaunchHook),
    AfterNativeCastUpdate(AfterNativeCastUpdateHook),
    AfterChainResolved(AfterChainResolvedHook),
    BeforePendingScriptEvents(BeforePendingScriptEventsHook),
    AfterPolicyCast(AfterPolicyCastLookup),
    BeforeMeleeComboDamage(BeforeMeleeComboDamageHook),
    BeforePeriodicItemDamage(BeforePeriodicItemDamageHook),
}

#[derive(Clone, Copy, Resource)]
pub struct SpellScriptRegistry {
    modules: &'static [&'static [SpellScriptHook]],
}

impl SpellScriptRegistry {
    pub const fn new(modules: &'static [&'static [SpellScriptHook]]) -> Self {
        Self { modules }
    }

    fn hooks(self) -> impl Iterator<Item = &'static SpellScriptHook> {
        self.modules.iter().flat_map(|module| module.iter())
    }

    pub fn declared_reach(self, world: &World, caster: Entity, spell: &SpellInfo) -> DeclaredReach {
        self.hooks()
            .fold(DeclaredReach::default(), |mut reach, hook| {
                if let SpellScriptHook::DeclaredReach(declare) = hook
                    && let ScriptReach::Area(declared) = declare(world, caster, spell)
                {
                    reach.caster_radius = reach.caster_radius.max(declared.caster_radius);
                    reach.target_radius = reach.target_radius.max(declared.target_radius);
                }
                reach
            })
    }

    pub fn additional_reach_actions(self, world: &World, unit: Entity) -> Vec<u32> {
        self.hooks()
            .filter_map(|hook| match hook {
                SpellScriptHook::AdditionalReachActions(actions) => Some(actions(world, unit)),
                _ => None,
            })
            .flatten()
            .collect()
    }

    pub fn effect_handler(self, spell: u32, index: u8) -> Option<super::effects::EffectHandler> {
        self.hooks().find_map(|hook| match hook {
            SpellScriptHook::EffectHandler(lookup) => lookup(spell, index),
            _ => None,
        })
    }

    pub fn filter_targets(
        self,
        world: &World,
        caster: Entity,
        spell: &SpellInfo,
        effect: u8,
        primary: Option<Entity>,
        units: &mut Vec<Entity>,
    ) {
        for hook in self.hooks() {
            if let SpellScriptHook::TargetFilter(run) = hook {
                run(world, caster, spell, effect, primary, units);
            }
        }
    }

    pub fn effect_hit(self, spell: u32, index: u8) -> Option<HitHook> {
        self.hooks().find_map(|hook| match hook {
            SpellScriptHook::EffectHit(lookup) => lookup(spell, index),
            _ => None,
        })
    }

    pub fn combat_status_changed(self, world: &mut World, unit: Entity, active: bool) {
        for hook in self.hooks() {
            if let SpellScriptHook::CombatStatusChanged(run) = hook {
                run(world, unit, active);
            }
        }
    }

    pub fn summon_initialized(self, world: &mut World, actor: Entity) {
        for hook in self.hooks() {
            if let SpellScriptHook::SummonInitialized(run) = hook {
                run(world, actor);
            }
        }
    }

    pub fn summon_expired(self, world: &mut World, actor: Entity) {
        for hook in self.hooks() {
            if let SpellScriptHook::SummonExpired(run) = hook {
                run(world, actor);
            }
        }
    }

    pub fn summon_removed(self, world: &mut World, actor: Entity) {
        for hook in self.hooks() {
            if let SpellScriptHook::SummonRemoved(run) = hook {
                run(world, actor);
            }
        }
    }

    pub fn register_early_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::EarlyAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn register_rogue_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::RogueAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn register_support_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::SupportAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn consumes_named_aura_field(self, spell: u32, index: u8) -> bool {
        self.hooks().any(|hook| match hook {
            SpellScriptHook::NamedAuraFields(values) => values.contains(&(spell, index)),
            _ => false,
        })
    }

    pub fn preserves_periodic_phase(self, spell: u32) -> bool {
        self.hooks().any(|hook| match hook {
            SpellScriptHook::PreservePeriodicPhase(spells) => spells.contains(&spell),
            _ => false,
        })
    }

    pub fn register_weapon_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::WeaponAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn counter_only_dot_stacks(self, spell: u32) -> bool {
        self.hooks().any(|hook| match hook {
            SpellScriptHook::CounterOnlyDotStacks(spells) => spells.contains(&spell),
            _ => false,
        })
    }

    pub fn consumes_effect_value(self, spell: u32, index: u8) -> bool {
        self.hooks().any(|hook| match hook {
            SpellScriptHook::ConsumedEffectValues(values) => values.contains(&(spell, index)),
            _ => false,
        })
    }

    pub fn is_cast_time_proc(self, spell: u32) -> bool {
        self.hooks().any(|hook| match hook {
            SpellScriptHook::CastTimeProcs(spells) => spells.contains(&spell),
            _ => false,
        })
    }

    pub fn preserve_unadmitted_proc(
        self,
        spell: &SpellInfo,
        aura: &crate::auras::AuraInstance,
    ) -> bool {
        !spell.cast_proc_mods.contains(&aura.instance_id)
            && self.hooks().any(|hook| match hook {
                SpellScriptHook::PreserveUnadmittedProcs(spells) => spells.contains(&aura.spell_id),
                _ => false,
            })
    }

    pub fn owns_policy_proc(self, spell: u32) -> bool {
        self.hooks().any(|hook| match hook {
            SpellScriptHook::OwnsPolicyProc(lookup) => lookup(spell),
            _ => false,
        })
    }

    pub fn register_late_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::LateAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn register_before_late_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::BeforeLateAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn register_mid_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::MidAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn register_fire_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::FireAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn policies(self) -> impl Iterator<Item = &'static Policy> {
        self.hooks().flat_map(|hook| match hook {
            SpellScriptHook::Policies(policies) => *policies,
            _ => &[],
        })
    }
}

impl Default for SpellScriptRegistry {
    fn default() -> Self {
        // Unit tests compile the exact downstream module sources against this test
        // crate's types. Production and external fixtures install the host registry.
        #[cfg(test)]
        {
            debug_assert_eq!(fixtures::NAMED_MODULES.len(), fixtures::MODULES.len());
            return Self::new(fixtures::MODULES);
        }
        #[cfg(not(test))]
        Self::new(&[])
    }
}

pub(crate) fn scripted_channel(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    world
        .get_resource::<SpellScriptRegistry>()
        .is_some_and(|registry| {
            registry.hooks().any(|hook| match hook {
                SpellScriptHook::ScriptedChannel(test) => test(world, caster, spell),
                _ => false,
            })
        })
}

pub(crate) fn requires_resource_context(world: &World, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::ResourceContext(spells)
            | SpellScriptHook::ResourceOnlyContext(spells) => spells.contains(&spell),
            _ => false,
        })
}

pub(crate) fn requires_target_context(world: &World, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::ResourceContext(spells) => spells.contains(&spell),
            _ => false,
        })
}

pub(crate) fn asynchronous_aura(world: &World, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::AsynchronousAuras(spells) => spells.contains(&spell),
            _ => false,
        })
}

pub(crate) fn incoming_aura_ignores_stacks(world: &World, spell: u32) -> bool {
    world
        .get_resource::<SpellScriptRegistry>()
        .is_some_and(|registry| {
            registry.hooks().any(|hook| match hook {
                SpellScriptHook::NonStackingIncomingAuras(spells) => spells.contains(&spell),
                _ => false,
            })
        })
}

/// Actual unresolved spell damage only; prediction never calls sampling hooks.
pub fn actual_damage_multiplier(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) -> f32 {
    let mut multiplier = 1.0;
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ActualDamageMultiplier(run) = hook {
            multiplier *= run(world, ctx, target);
        }
    }
    multiplier
}

/// Pure expected multipliers for the existing read-only damage snapshot API.
pub fn expected_damage_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .map(|hook| match hook {
            SpellScriptHook::ExpectedDamageMultiplier(run) => run(world, caster, target, spell),
            _ => 1.0,
        })
        .product()
}

pub fn policies(world: &World) -> impl Iterator<Item = &'static Policy> + use<> {
    world.resource::<SpellScriptRegistry>().policies()
}

pub fn native_after_cast(world: &World, spell: u32) -> Option<AfterCastHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::NativeAfterCast(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn aura_roll_after_cast(
    world: &World,
    spell: u32,
) -> impl Iterator<Item = AfterCastHook> + use<> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::AuraRollAfterCast(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::OnCast(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

pub fn after_payment(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterPayment(run) = hook {
            run(world, caster, costs);
        }
    }
}

pub fn after_resource_values(
    world: &World,
    spell: u32,
) -> impl Iterator<Item = AfterCastHook> + use<> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::AfterResourceValues(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn target_plan(world: &World, spell: u32) -> Option<(Relation, TargetPlanHook)> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::TargetPlan(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn periodic_primary_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    amount: f32,
    crit: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::PeriodicPrimaryDamage(run) = hook {
            run(world, ctx, target, amount, crit);
        }
    }
}

pub fn periodic_tick_rate(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::PeriodicTickRate(run) => Some(run(world, caster, target, spell)),
            _ => None,
        })
        .product()
}

pub fn target_damage_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::TargetDamageMultiplier(run) => Some(run(world, caster, target, spell)),
            _ => None,
        })
        .product()
}

pub fn after_spender_stacks(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterSpenderStacks(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn aura_redirect(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: u32,
    depth: u8,
) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::AuraRedirect(run) => run(world, caster, target, spell, depth),
            _ => false,
        })
}

pub fn hit(world: &World, spell: u32) -> Option<HitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::Hit(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn context_hit(world: &World, spell: u32) -> Option<ContextHitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::ContextHit(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn area_filter(world: &World, spell: u32, index: u8) -> Option<AreaTargetFilter> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::AreaFilter(lookup) => lookup(spell, index),
            _ => None,
        })
}

pub fn check_target(
    world: &World,
    caster: Entity,
    spell: u32,
    target: Entity,
) -> Result<(), super::CastFailure> {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::CheckTarget(run) = hook {
            run(world, caster, spell, target)?;
        }
    }
    Ok(())
}

pub fn damage_amount(world: &World, spell: u32) -> Option<DamageAmountHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::DamageAmount(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn auto_repeat(world: &mut World, caster: Entity, target: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AutoRepeat(run) = hook {
            run(world, caster, target);
        }
    }
}

/// Live source-specific multipliers on actual critical damage.
pub fn actual_critical_damage_multiplier(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::ActualCriticalDamageMultiplier(run) => Some(run(world, caster, spell)),
            _ => None,
        })
        .product()
}

/// Sample additive critical-chance bonuses for an actual damage impact, not forecasts.
pub fn actual_critical_chance_bonus(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
    chance: f32,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::ActualCriticalChanceBonus(run) => {
                Some(run(world, caster, target, spell, chance))
            }
            _ => None,
        })
        .sum()
}

pub fn critical_chance(world: &World, caster: Entity, target: Entity, spell: u32) -> Option<f32> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::CriticalChance(run) => run(world, caster, target, spell),
            _ => None,
        })
}

pub fn launch_delay(world: &World, spell: u32, native: f32) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::LaunchDelay(lookup) => lookup(spell),
            _ => None,
        })
        .unwrap_or(native)
}

pub fn dot_mastery_multiplier(world: &World, caster: Entity, target: Entity, spell: u32) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::DotMasteryMultiplier(run) => Some(run(world, caster, target, spell)),
            _ => None,
        })
        .product()
}

pub fn periodic_damage_finished(world: &mut World, ctx: &EffectCtx<'_>) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::PeriodicDamageFinished(run) = hook {
            run(world, ctx);
        }
    }
}

pub fn after_foreground_special(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterForegroundSpecial(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn after_counter_launch(world: &mut World, caster: Entity, spell: u32) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterCounterLaunch(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn impact_proc_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ImpactProcDamage(run) = hook {
            run(world, ctx, target);
        }
    }
}

/// Every resolved white impact, including avoided or immune outcomes.
/// Existing landed-hit producers retain their separate stage.
pub fn after_white_impact(world: &mut World, caster: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterWhiteImpact(run) = hook {
            run(world, caster);
        }
    }
}

pub fn combo_point_gain(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ComboPointGain(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

pub fn after_white_hand_hit(
    world: &mut World,
    caster: Entity,
    hand: crate::melee::Hand,
    critical: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterWhiteHandHit(run) = hook {
            run(world, caster, hand, critical);
        }
    }
}

pub fn after_white_hit(world: &mut World, caster: Entity, critical: bool) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterWhiteHit(run) = hook {
            run(world, caster, critical);
        }
    }
}

pub fn replacement_direct_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ReplacementDirectDamage(run) = hook {
            run(world, ctx, target, critical);
        }
    }
}

pub fn resource_value_after_cast(
    world: &World,
    spell: u32,
) -> impl Iterator<Item = AfterCastHook> + use<> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::ResourceValueAfterCast(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn triggered_resource_producer(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::TriggeredResourceProducer(run) = hook {
            run(world, caster, spell);
        }
    }
}

/// Source-script cost adjustments run after native aura modifiers, before payment.
/// Without a registered callback, the native cost is unchanged.
pub fn power_cost(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    pool: CostPool,
    amount: f32,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .fold(amount, |cost, hook| match hook {
            SpellScriptHook::PowerCost(run) => run(world, caster, spell, pool, cost),
            _ => cost,
        })
}

pub fn effective_resource_value(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    resource: shared::components::PowerType,
    paid: f32,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .fold(paid, |value, hook| match hook {
            SpellScriptHook::EffectiveResourceValue(run) => {
                run(world, caster, spell, resource, value)
            }
            _ => value,
        })
}

pub fn consume_resource_value_sources(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ConsumeResourceValueSources(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn spell_residual(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, amount: f32) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::SpellResidual(run) = hook {
            run(world, ctx, target, amount);
        }
    }
}

pub fn melee_residual(world: &mut World, caster: Entity, target: Entity, amount: f32) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::MeleeResidual(run) = hook {
            run(world, caster, target, amount);
        }
    }
}

pub fn avoidance_proc_hit_mask(world: &World, spell: u32) -> Option<u32> {
    world
        .get_resource::<SpellScriptRegistry>()?
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::AvoidanceProcHitMask(run) => run(spell),
            _ => None,
        })
}

pub fn after_absorb_consumed(
    world: &mut World,
    caster: Entity,
    victim: Entity,
    attacker: Option<Entity>,
    spell: u32,
    amount: f32,
) {
    let Some(registry) = world.get_resource::<SpellScriptRegistry>().copied() else {
        return;
    };
    for hook in registry.hooks() {
        if let SpellScriptHook::AfterAbsorbConsumed(run) = hook {
            run(world, caster, victim, attacker, spell, amount);
        }
    }
}

pub fn successful_cast_relationships(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::SuccessfulCastRelationships(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn guarantees_crit(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::GuaranteesCrit(test) => test(world, caster, spell),
            _ => false,
        })
}

pub fn allows_cast_during_channel(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    world
        .get_resource::<SpellScriptRegistry>()
        .is_some_and(|registry| {
            registry.hooks().any(|hook| {
        matches!(hook, SpellScriptHook::AllowCastDuringChannel(test) if test(world, caster, spell))
    })
        })
}

pub fn ignores_cooldown(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::IgnoresCooldown(test) => test(world, caster, spell),
            _ => false,
        })
}

pub fn snapshots_modifier(world: &World, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::SnapshotsModifier(test) => test(spell),
            _ => false,
        })
}

pub fn uses_persistent_cast_multiplier(world: &World, spell: u32) -> bool {
    world
        .get_resource::<SpellScriptRegistry>()
        .is_some_and(|registry| {
            registry.hooks().any(|hook| match hook {
                SpellScriptHook::PersistentCastMultiplier(spells) => spells.contains(&spell),
                _ => false,
            })
        })
}

pub fn ignores_armor(world: &World, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::ArmorIgnoredSpells(spells) => spells.contains(&spell),
            _ => false,
        })
}

pub fn melee_raw_resolved(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    amount: f32,
    crit: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::MeleeRawResolved(run) = hook {
            run(world, attacker, target, amount, crit);
        }
    }
}

pub fn raw_resolved_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
    amount: f32,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::RawResolvedDamage(run) = hook {
            run(world, ctx, target, critical, amount);
        }
    }
}

pub fn resolved_heal(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    raw: f32,
    effective: f32,
    critical: bool,
    periodic: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ResolvedHeal(run) = hook {
            run(world, ctx, target, raw, effective, critical, periodic);
        }
    }
}

pub fn resolved_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
    amount: f32,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ResolvedDamage(run) = hook {
            run(world, ctx, target, critical, amount);
        }
    }
}

pub fn after_direct_damage_before_items(world: &mut World, ctx: &EffectCtx<'_>) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterDirectDamageBeforeItems(run) = hook {
            run(world, ctx);
        }
    }
}

pub fn after_aura_application(
    world: &mut World,
    target: Entity,
    aura: &crate::auras::AuraInstance,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterAuraApplication(run) = hook {
            run(world, target, aura);
        }
    }
}

pub fn landed_damage_policies(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
    periodic: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::LandedDamagePolicy(run) = hook {
            run(world, caster, target, spell, amount, periodic);
        }
    }
}

pub fn action_available(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .all(|hook| match hook {
            SpellScriptHook::ActionAvailable(test) => test(world, caster, spell),
            _ => true,
        })
}

pub fn ability_use_available(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::AbilityUseAvailable(test) => test(world, caster, spell),
            _ => false,
        })
}

pub fn channel_periodic(world: &World, spell: u32, index: u8) -> Option<ContextHitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::ChannelPeriodic(lookup) => lookup(spell, index),
            _ => None,
        })
}

pub fn after_periodic_energize(world: &mut World, unit: Entity, spell: u32, effect: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterPeriodicEnergize(run) = hook {
            run(world, unit, spell, effect);
        }
    }
}

pub fn before_native_on_cast(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeNativeOnCast(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

pub fn prepare_stored_damage(world: &World, spell: &mut SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::PrepareStoredDamage(run) = hook {
            run(spell);
        }
    }
}

pub fn enemy_context_hit(world: &World, spell: u32) -> Option<EnemyContextHitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::EnemyContextHit(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn after_policy_direct_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterPolicyDirectDamage(run) = hook {
            run(world, ctx, target, critical);
        }
    }
}

pub fn after_policy_periodic_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterPolicyPeriodicDamage(run) = hook {
            run(world, ctx, target, critical);
        }
    }
}

pub fn target_context_damage_multiplier(
    world: &World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    periodic: bool,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::TargetContextDamageMultiplier(run) => {
                Some(run(world, ctx, target, periodic))
            }
            _ => None,
        })
        .product()
}

pub fn after_combo_overflow(world: &mut World, target: Entity, overflow: f32) {
    let hooks: Vec<_> = world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::AfterComboOverflow(run) => Some(run),
            _ => None,
        })
        .collect();
    for run in hooks {
        run(world, target, overflow);
    }
}

pub fn after_combo_gain(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, gained: f32) {
    let hooks: Vec<_> = world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::AfterComboGain(run) => Some(run),
            _ => None,
        })
        .collect();
    for run in hooks {
        run(world, ctx, target, gained);
    }
}

pub fn after_native_on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterNativeOnCast(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

pub fn after_modifier_consumption(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterModifierConsumption(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn after_foreground_spender(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterForegroundSpender(run) = hook {
            run(world, caster, spell, target);
        }
    }
}

pub fn after_spender_proc(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterSpenderProc(run) = hook {
            run(world, caster, spell, target);
        }
    }
}

pub fn after_elemental_launch(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    foreground: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterElementalLaunch(run) = hook {
            run(world, caster, spell, foreground);
        }
    }
}

pub fn before_direct_damage_policies(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeDirectDamagePolicies(run) = hook {
            run(world, ctx, target, critical);
        }
    }
}

pub fn after_periodic_damage_policies(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterPeriodicDamagePolicies(run) = hook {
            run(world, ctx, target);
        }
    }
}

pub fn empower_completion(world: &World, spell: u32) -> Option<EmpowerCompleteHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::EmpowerCompletion(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn empower_damage_bonus(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::EmpowerDamageBonus(run) => Some(run(world, caster, spell)),
            _ => None,
        })
        .sum()
}

pub fn empower_release_preview(
    world: &World,
    caster: Entity,
    spell: u32,
    rank: u8,
) -> Option<SpellInfo> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::EmpowerReleasePreview(run) => run(world, caster, spell, rank),
            _ => None,
        })
}

pub fn capture_consumed_damage_modifier(
    world: &World,
    caster: Entity,
    spell: &mut SpellInfo,
    foreground: bool,
) -> Option<u32> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::CaptureConsumedDamageModifier(capture) => {
                capture(world, caster, spell, foreground)
            }
            _ => None,
        })
}

pub fn finish_consumed_damage_modifier(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    consumed: Option<u32>,
    depth: u8,
    foreground: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::FinishConsumedDamageModifier(run) = hook {
            run(world, caster, spell, consumed, depth, foreground);
        }
    }
}

pub fn before_judgment_on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeJudgmentOnCast(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

pub fn leech_heal_override(world: &World, ctx: &EffectCtx<'_>, damage: f32) -> Option<f32> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::LeechHealOverride(run) => run(world, ctx, damage),
            _ => None,
        })
}

pub fn healing_multiplier(world: &World, caster: Entity, spell: &SpellInfo, periodic: bool) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::HealingMultiplier(run) => Some(run(world, caster, spell, periodic)),
            _ => None,
        })
        .product()
}

pub fn snapshot_healing_aura(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &crate::spell_info::SpellEffect,
    amount: f32,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .fold(amount, |value, hook| match hook {
            SpellScriptHook::SnapshotHealingAura(run) => run(world, caster, spell, effect, value),
            _ => value,
        })
}

pub fn triggers_hot_streak(world: &World, spell: u32) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::HotStreakProducers(spells) => spells.contains(&spell),
            _ => false,
        })
}

pub fn before_proc_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeProcLaunch(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn after_fire_direct_hit(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterFireDirectHit(run) = hook {
            run(world, ctx, target, critical);
        }
    }
}

pub fn after_fire_outcome_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterFireOutcomeHit(run) = hook {
            run(world, ctx, target);
        }
    }
}

pub fn record_fire_outcome(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    critical: bool,
    guaranteed: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::RecordFireOutcome(run) = hook {
            run(world, caster, spell, critical, guaranteed);
        }
    }
}

pub fn cast_hit_proc_chance(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    hit: Option<Entity>,
) -> Option<f32> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::CastHitProcChance(run) => run(world, caster, spell, hit),
            _ => None,
        })
}

pub fn roll_cast_hit_proc(world: &mut World, caster: Entity, chance: f32) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::RollCastHitProc(run) = hook {
            run(world, caster, chance);
        }
    }
}

pub fn launch_damage_modifiers<'a>(
    world: &'a World,
    caster: Entity,
    spell: &'a SpellInfo,
) -> impl Iterator<Item = crate::spell_info::ChargedSpellModifier> + 'a {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::LaunchDamageModifier(run) => run(world, caster, spell),
            _ => None,
        })
}

pub fn after_queued_triggers(world: &mut World) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterQueuedTriggers(run) = hook {
            run(world);
        }
    }
}

pub fn after_spell_payment(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    costs: &[(CostPool, f32)],
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterSpellPayment(run) = hook {
            run(world, caster, spell, costs);
        }
    }
}

pub fn before_damage_snapshot(world: &World, caster: Entity, spell: &mut SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeDamageSnapshot(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn before_execution_counter_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeExecutionCounterLaunch(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn policy_tail_cast(world: &World, spell: u32) -> Option<PolicyTailCastHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::PolicyTailCast(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn policy_tail_hit(world: &World, spell: u32) -> Option<PolicyTailHitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::PolicyTailHit(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn proc_charges(world: &World, caster: Entity, spell: &SpellInfo, native: f32) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .fold(native, |amount, hook| match hook {
            SpellScriptHook::ProcCharges(run) => run(world, caster, spell, amount),
            _ => amount,
        })
}

pub fn extra_combo_points(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::ExtraComboPoints(run) => Some(run(world, caster, spell)),
            _ => None,
        })
        .sum()
}

pub fn named_healing_multiplier(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::NamedHealingMultiplier(run) => Some(run(world, caster, spell)),
            _ => None,
        })
        .product()
}

pub fn guardian_action_eligible(world: &World, actor: Entity, spell: u32, target: Entity) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .all(|hook| match hook {
            SpellScriptHook::GuardianActionEligible(run) => run(world, actor, spell, target),
            _ => true,
        })
}

pub fn effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .all(|hook| match hook {
            SpellScriptHook::EffectEnabled(run) => run(world, caster, spell, index),
            _ => true,
        })
}

pub fn check_native_spell(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), super::CastFailure> {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::CheckNativeSpell(run) = hook {
            run(world, caster, spell)?;
        }
    }
    Ok(())
}

pub fn after_resolved_hits(world: &mut World, ctx: &EffectCtx<'_>) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterResolvedHits(run) = hook {
            run(world, ctx);
        }
    }
}

pub fn snapshot_native_aura_amount(
    world: &World,
    spell: &SpellInfo,
    index: u8,
    native: f32,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .fold(native, |amount, hook| match hook {
            SpellScriptHook::SnapshotNativeAuraAmount(run) => run(world, spell, index, amount),
            _ => amount,
        })
}

pub fn area_trigger_expired(world: &mut World, caster: Entity, spell: u32, position: Vec3) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AreaTriggerExpired(run) = hook {
            run(world, caster, spell, position);
        }
    }
}

pub fn after_cast_update(world: &mut World) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterCastUpdate(run) = hook {
            run(world);
        }
    }
}

pub fn consume_cast_modifiers(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ConsumeCastModifiers(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn after_weapon_launch(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterWeaponLaunch(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

pub fn after_policy_cast(world: &World, spell: u32) -> Option<AfterPolicyCastHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::AfterPolicyCast(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn before_melee_combo_damage(world: &mut World, ctx: &EffectCtx<'_>) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeMeleeComboDamage(run) = hook {
            run(world, ctx);
        }
    }
}

pub fn before_periodic_item_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforePeriodicItemDamage(run) = hook {
            run(world, ctx, target);
        }
    }
}

pub fn conjure_count(world: &mut World, caster: Entity, target: Entity, count: u32) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ConjureCount(run) = hook {
            run(world, caster, target, count);
        }
    }
}

/// Source scripts can require the existing paid-resource and primary snapshot.
pub fn policy_context_required(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::PolicyContextRequired(run) => run(world, caster, spell),
            _ => false,
        })
}

/// Script snapshots follow captured policy context and precede ordinary aura preparation.
pub fn prepare_policy_spell(world: &World, caster: Entity, spell: &mut SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::PreparePolicySpell(run) = hook {
            run(world, caster, spell);
        }
    }
}

/// The policy dispatcher has already checked the original primary before this slot.
pub fn before_policy_primary_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforePolicyPrimaryHit(run) = hook {
            run(world, ctx, target);
        }
    }
}

/// First native enemy-hit callback, before ordinary registered enemy impact lookups.
pub fn before_enemy_context_hit(world: &World, spell: u32) -> Option<BeforeEnemyContextHitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::BeforeEnemyContextHit(lookup) => lookup(spell),
            _ => None,
        })
}

/// Report a resolved heal to recipient-owned effects, including overheal.
pub fn heal_received(
    world: &mut World,
    target: Entity,
    spell: u32,
    raw: f32,
    effective: f32,
    percent_based: bool,
) {
    let registry = *world.resource::<SpellScriptRegistry>();
    for hook in registry.hooks() {
        if let SpellScriptHook::HealReceived(run) = hook {
            run(world, target, spell, raw, effective, percent_based);
        }
    }
}

/// Report landed source damage before target mitigation and absorbs.
pub fn incoming_damage_before_mitigation(
    world: &mut World,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
) {
    if amount <= 0.0 {
        return;
    }
    let registry = *world.resource::<SpellScriptRegistry>();
    for hook in registry.hooks() {
        if let SpellScriptHook::IncomingDamageBeforeMitigation(run) = hook {
            run(world, target, spell, amount);
        }
    }
}

/// Consume earned healing before Expel Harm calculates its damage payload.
pub fn expel_harm_heal(world: &mut World, caster: Entity) -> f32 {
    let registry = *world.resource::<SpellScriptRegistry>();
    registry
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::ExpelHarmHeal(run) => Some(run(world, caster)),
            _ => None,
        })
        .sum()
}

/// Observe healing after source and target multipliers but before critical and partial-tick scaling.
pub fn raw_heal(world: &mut World, ctx: &EffectCtx<'_>, amount: f32) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::RawHeal(run) = hook {
            run(world, ctx, amount);
        }
    }
}

pub fn aura_event(world: &mut World, event: &crate::auras::procs::AuraEvent) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AuraEvent(run) = hook {
            run(world, event);
        }
    }
}

/// Notify scripts of a landed spell impact with the target's exact pre-impact health.
pub fn spell_health_impact(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    before: f32,
    healing: bool,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::SpellHealthImpact(run) = hook {
            run(world, ctx, target, before, healing);
        }
    }
}

/// Health history observes the emitted event after meters and arena bookkeeping.
pub fn health_change_observed(world: &mut World, event: &shared::protocol::CombatLogEvent) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::HealthChangeObserved(run) = hook {
            run(world, event);
        }
    }
}

/// Admission-time proc markers, before payment and missile flight.
pub fn snapshot_launch_auras(world: &World, caster: Entity, spell: &mut SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::SnapshotLaunchAuras(run) = hook {
            run(world, caster, spell);
        }
    }
}

/// First authored scalar for a triggered child, retaining the parent's snapshot.
pub fn triggered_child_damage_multiplier(
    world: &World,
    caster: Entity,
    parent: &SpellInfo,
    child: u32,
) -> Option<f32> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::TriggeredChildDamageMultiplier(run) => {
                run(world, caster, parent, child)
            }
            _ => None,
        })
}

/// Execution proc consumers run after native handlers and before supply producers.
pub fn proc_after_cast(world: &World, spell: u32) -> impl Iterator<Item = AfterCastHook> + use<> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::ProcAfterCast(lookup) => lookup(spell),
            _ => None,
        })
}

/// Supply producers run before generic successful-cast policy relationships.
pub fn supply_after_cast(world: &World, spell: u32) -> impl Iterator<Item = AfterCastHook> + use<> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::SupplyAfterCast(lookup) => lookup(spell),
            _ => None,
        })
}

/// Resolve script-owned delays after queued triggers and before their cleanup.
pub fn resolve_pending_script_events(world: &mut World) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ResolvePendingScriptEvents(run) = hook {
            run(world);
        }
    }
}

/// Launch supply follows modifier consumption and precedes execution counters.
pub fn after_launch_proc_supply(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterLaunchProcSupply(run) = hook {
            run(world, caster, spell);
        }
    }
}

/// Source-preserving explicit conjures from an upstream native caller.
pub fn conjure_from_spell(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Entity,
    count: u32,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::ConjureFromSpell(run) = hook {
            run(world, caster, spell, target, count);
        }
    }
}

/// Native root snapshot before the ordinary on-cast registry.
pub fn before_script_on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforeScriptOnCast(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

/// Actual weapon impact at the original pre-policy position.
pub fn after_weapon_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, amount: f32) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterWeaponDamage(run) = hook {
            run(world, ctx, target, amount);
        }
    }
}

pub fn native_weapon_multiplier(
    world: &World,
    caster: Entity,
    parent: &SpellInfo,
    child: u32,
) -> Option<f32> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::NativeWeaponMultiplier(run) => run(world, caster, parent, child),
            _ => None,
        })
}

/// Native repeated launch after the upstream Arcane launch callback.
pub fn after_native_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterNativeLaunch(run) = hook {
            run(world, caster, spell);
        }
    }
}

/// Independent scheduler retains its own after-advance_casts system boundary.
pub fn after_native_cast_update(world: &mut World) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterNativeCastUpdate(run) = hook {
            run(world);
        }
    }
}

/// Actual chain recipients are still captured before the resolving frame is removed.
pub fn after_chain_resolved(world: &mut World, ctx: &EffectCtx<'_>) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterChainResolved(run) = hook {
            run(world, ctx);
        }
    }
}

/// Retain native discharge resolution before Frost and generic queued events.
pub fn before_pending_script_events(world: &mut World) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::BeforePendingScriptEvents(run) = hook {
            run(world);
        }
    }
}

#[cfg(test)]
use crate as combat;
#[cfg(test)]
#[path = "../../../server-spell-scripts/src/modules.rs"]
mod fixtures;

// Native unit fixtures compile the same helper against their own combat types.
#[cfg(test)]
#[path = "../../../server-spell-scripts/src/bombardments.rs"]
pub mod bombardments;
