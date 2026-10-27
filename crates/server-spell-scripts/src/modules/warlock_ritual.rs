//! Demonology Diabolist: paid Hand → Diabolic Ritual → Demonic Art → demon summon,
//! with the summon-granted Ruination and Infernal Bolt replacements.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::powers::PowerTypeTable;
use combat::spell_cast::script_registry::{EnemyContextHitHook, SpellScriptHook};
use combat::spell_cast::{self, CostPool};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, CombatStatus, PowerType};

const RITUAL: u32 = 428514;
const HAND: u32 = 105174;
const RUINATION_TALENT: u32 = 428522;
const RUINATION_BUFF: u32 = 433885;
const RUINATION_CAST: u32 = 434635;
const RUINATION_IMPACT: u32 = 434636;
const COVEN: u32 = 428518;
const INFERNAL_BOLT_BUFF: u32 = 433891;
const INFERNAL_BOLT: u32 = 434506;
const DIABOLIC_OCULI: u32 = 1268709;
const DEMONIC_OCULI: u32 = 1269643;
const EYE_EXPLOSION: u32 = 1269800;
const LOOKS_THAT_KILL: u32 = 1268713;
const GAZES: [u32; 3] = [1269892, 1269886, 1269885];
const MINDS_EYES_TALENT: u32 = 1268716;
const MINDS_EYES: u32 = 1269879;

/// SimC76f8118 init.cpp927–931: each ritual earns its Art and names the next ritual.
struct Demon {
    ritual: u32,
    art: u32,
    summon: u32,
}

const DEMONS: [Demon; 3] = [
    Demon {
        ritual: 431944,
        art: 428524,
        summon: 428571,
    }, // Overlord → Mother
    Demon {
        ritual: 432815,
        art: 432794,
        summon: 428565,
    }, // Mother → Pit Lord
    Demon {
        ritual: 432816,
        art: 432795,
        summon: 434400,
    }, // Pit Lord → Overlord
];

/// The cast's primary: the triggered impact child does not carry it.
#[derive(Component)]
struct RuinationTarget(Entity);

/// Index into `DEMONS` of the ritual the next unritualed paid Hand starts.
#[derive(Component)]
struct RitualCycle(usize);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterSpellPayment(after_payment),
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::EnemyContextHit(ruination_hit),
    SpellScriptHook::TargetDamageMultiplier(explosion_multiplier),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(DEMONIC_OCULI).or_default().on_periodic = Some(gaze);
    for demon in &DEMONS {
        scripts
            .0
            .entry(demon.ritual)
            .or_insert(AuraScript::default())
            .on_natural_end = Some(ritual_ended);
        scripts
            .0
            .entry(demon.art)
            .or_insert(AuraScript::default())
            .on_remove = Some(art_removed);
    }
}

fn diabolist(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 266)
        && auras::has_aura(world, caster, RITUAL)
}

fn in_combat(world: &World, unit: Entity) -> bool {
    world
        .get::<CombatStatus>(unit)
        .is_some_and(|status| status.0)
}

fn after_payment(world: &mut World, caster: Entity, spell: &SpellInfo, costs: &[(CostPool, f32)]) {
    if !diabolist(world, caster) {
        return;
    }
    match spell.id {
        HAND => {
            // SimC actions.cpp2840–2841: each Hand execution adds a Demonic Oculi.
            if auras::has_aura(world, caster, DIABOLIC_OCULI) {
                grant(world, caster, DEMONIC_OCULI, 1);
            }
            // actions.cpp2810–2818/2849–2851: Art consumption needs the Art at cast start.
            let cast_start = spell_cast::now(world)
                - f64::from(spell_cast::cast_time_ms(world, caster, spell).max(0)) / 1000.0;
            if consume_art(world, caster, Some(cast_start)) {
                detonate_oculi(world, caster, spell.cast_primary_target);
            }
            if in_combat(world, caster) {
                advance_ritual(world, caster, costs);
            }
        }
        // actions.cpp5320–5333: Ruination consumes any present Art and one buff stack.
        RUINATION_CAST => {
            if let Some(primary) = spell.cast_primary_target {
                world.entity_mut(caster).insert(RuinationTarget(primary));
            }
            if consume_art(world, caster, None) {
                detonate_oculi(world, caster, spell.cast_primary_target);
            }
            spend_own_stack(world, caster, RUINATION_BUFF);
        }
        // actions.cpp5233–5240.
        INFERNAL_BOLT => spend_own_stack(world, caster, INFERNAL_BOLT_BUFF),
        _ => {}
    }
}

fn own_aura(world: &World, caster: Entity, spell: u32) -> Option<&AuraInstance> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
}

fn spend_own_stack(world: &mut World, caster: Entity, spell: u32) {
    if let Some(instance) = own_aura(world, caster, spell).map(|aura| aura.instance_id) {
        auras::spend_stack(world, caster, instance);
    }
}

/// Spend the held Art; `granted_by` excludes an Art earned after that time.
fn consume_art(world: &mut World, caster: Entity, granted_by: Option<f64>) -> bool {
    let held = DEMONS.iter().find_map(|demon| {
        let aura = own_aura(world, caster, demon.art)?;
        let granted = aura.expires_at? - f64::from(aura.duration_ms) / 1000.0;
        granted_by
            .is_none_or(|limit| granted <= limit + 1e-6)
            .then_some(aura.instance_id)
    });
    if let Some(instance) = held {
        auras::spend_stack(world, caster, instance);
    }
    held.is_some()
}

fn grant(world: &mut World, caster: Entity, buff: u32, depth: u8) {
    if !spell_cast::try_cast_triggered(world, caster, buff, Some(caster), depth) {
        warn!(?caster, buff, "Native Diabolist buff refused");
    }
}

fn own_stacks(world: &World, caster: Entity, spell: u32) -> u8 {
    own_aura(world, caster, spell).map_or(0, |aura| aura.stacks.max(1))
}

/// actions.cpp370–403/5356–5376: an Art-consuming cast grants one Mind's Eyes per
/// held Oculi, explodes them on its target scaled by that count, then expires them.
fn detonate_oculi(world: &mut World, caster: Entity, target: Option<Entity>) {
    let stacks = own_stacks(world, caster, DEMONIC_OCULI);
    let Some(target) = target.filter(|_| stacks > 0) else {
        return;
    };
    if auras::has_aura(world, caster, MINDS_EYES_TALENT) {
        for _ in 0..stacks {
            grant(world, caster, MINDS_EYES, 1);
        }
    }
    spell_cast::cast_triggered(world, caster, EYE_EXPLOSION, Some(target), 1);
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == DEMONIC_OCULI && aura.caster == caster
    });
}

// init.cpp940–958: each unhasted Oculi tick casts the Diabolic Gaze matching its stacks.
fn gaze(world: &mut World, caster: Entity, aura: &AuraInstance, index: u8) {
    if index != 1 || aura.caster != caster || !auras::has_aura(world, caster, LOOKS_THAT_KILL) {
        return;
    }
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    else {
        return;
    };
    let spell = match aura.stacks {
        1 => GAZES[0],
        2 => GAZES[1],
        3 => GAZES[2],
        _ => return,
    };
    spell_cast::cast_triggered(world, caster, spell, Some(target), 1);
}

/// actions.cpp5356–5363: Eye Explosion damage scales with the held Oculi.
fn explosion_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.id == EYE_EXPLOSION {
        f32::from(own_stacks(world, caster, DEMONIC_OCULI))
    } else {
        1.0
    }
}

fn advance_ritual(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    // SimC actions.cpp302–355: shorten the held ritual by effect1 seconds per shard,
    // or start the cycle's next ritual and shorten it.
    let paid = spell_cast::spent(costs, PowerType::SoulShards);
    if paid <= 0.0 {
        return;
    }
    let units = world
        .resource::<PowerTypeTable>()
        .get(PowerType::SoulShards)
        .expect("native Soul Shards display units")
        .display_modifier;
    assert!(units > 0, "Soul Shards display divisor must be positive");
    let mut seconds = f64::from(spell_cast::scripts::script_effect_value(
        world, caster, RITUAL, 0,
    )) * f64::from(paid)
        / f64::from(units);
    // SimC76f8118 actions.cpp311–312: the native time bonus applies once
    // per spender while an actual owned Demonic Tyrant is active.
    if auras::has_aura(world, caster, 429917) {
        let tyrant_active = world
            .query::<(Entity, &spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .any(|(actor, summon)| {
                summon.owner == caster
                    && summon.spell_id == 265187
                    && spell_cast::targets::is_alive(world, actor)
            });
        if tyrant_active {
            seconds += f64::from(spell_cast::scripts::script_effect_value(
                world, caster, 429917, 0,
            )) / 1000.0;
        }
    }
    let held = DEMONS
        .iter()
        .find(|demon| own_aura(world, caster, demon.ritual).is_some());
    let ritual = match held {
        Some(demon) => demon.ritual,
        None => {
            let next = next_index(world, caster);
            let ritual = DEMONS[next].ritual;
            if !spell_cast::try_cast_triggered(world, caster, ritual, Some(caster), 1) {
                warn!(?caster, ritual, "Native Diabolic Ritual refused");
                return;
            }
            ritual
        }
    };
    let now = spell_cast::now(world);
    let expiry = own_aura(world, caster, ritual)
        .and_then(|aura| aura.expires_at)
        .expect("native ritual needs a finite expiry");
    let remaining_ms = ((expiry - now - seconds).max(0.0) * 1000.0).round() as u32;
    assert!(auras::set_owned_duration(
        world,
        caster,
        caster,
        ritual,
        remaining_ms
    ));
}

/// init.cpp1750: the first ritual of an encounter is uniformly chosen.
fn next_index(world: &mut World, caster: Entity) -> usize {
    if let Some(cycle) = world.get::<RitualCycle>(caster) {
        return cycle.0;
    }
    let first = combat::random::thread_rng().gen_range(0..DEMONS.len());
    world.entity_mut(caster).insert(RitualCycle(first));
    first
}

fn ritual_ended(world: &mut World, caster: Entity, aura: &AuraInstance) {
    let Some(index) = DEMONS
        .iter()
        .position(|demon| demon.ritual == aura.spell_id)
    else {
        return;
    };
    world
        .entity_mut(caster)
        .insert(RitualCycle((index + 1) % DEMONS.len()));
    let spell = world
        .resource::<SpellStore>()
        .get(aura.spell_id)
        .expect("native ritual spell")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    // init.cpp873–886: the Art arrives after a gauss(200ms, 15ms) delay.
    let delay = gauss_ms(200.0, 15.0);
    let grant: fn(&mut World, Entity, Entity, u8) = match index {
        0 => grant_art::<0>,
        1 => grant_art::<1>,
        _ => grant_art::<2>,
    };
    spell_cast::queued_triggers::schedule_callback(world, &ctx, caster, delay, grant);
}

fn grant_art<const INDEX: usize>(world: &mut World, caster: Entity, _: Entity, depth: u8) {
    // init.cpp876–885: a new Art replaces any held Art without summoning its demon.
    auras::remove_auras(world, caster, RemoveReason::Replaced, |aura| {
        aura.caster == caster && DEMONS.iter().any(|demon| demon.art == aura.spell_id)
    });
    let art = DEMONS[INDEX].art;
    if !spell_cast::try_cast_triggered(world, caster, art, Some(caster), depth) {
        warn!(?caster, art, "Native Demonic Art refused");
    }
}

fn art_removed(world: &mut World, caster: Entity, aura: &AuraInstance, reason: RemoveReason) {
    // init.cpp897–925: consumption or expiry in combat summons; replacement does not.
    if !matches!(reason, RemoveReason::Consumed | RemoveReason::Expired)
        || aura.caster != caster
        || !in_combat(world, caster)
    {
        return;
    }
    let Some(demon) = DEMONS.iter().find(|demon| demon.art == aura.spell_id) else {
        return;
    };
    if !spell_cast::try_cast_triggered(world, caster, demon.summon, Some(caster), 1) {
        warn!(
            ?caster,
            summon = demon.summon,
            "Native Diabolist summon refused"
        );
        return;
    }
    // actions.cpp5455–5458/5476–5481: Mother grants Infernal Bolt, Pit Lord Ruination.
    let granted = match demon.summon {
        428565 => (COVEN, INFERNAL_BOLT_BUFF),
        434400 => (RUINATION_TALENT, RUINATION_BUFF),
        _ => return,
    };
    if auras::has_aura(world, caster, granted.0)
        && !spell_cast::try_cast_triggered(world, caster, granted.1, Some(caster), 1)
    {
        warn!(
            ?caster,
            buff = granted.1,
            "Native Diabolist summon buff refused"
        );
    }
}

fn ruination_hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == RUINATION_IMPACT).then_some(launch_ruination_meteor)
}

fn launch_ruination_meteor(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // actions.cpp5276–5305: the primary impact drops a three-shard Hand meteor
    // after a gauss(395ms, 35ms) delay; that meteor summons the Wild Imps.
    if world
        .get::<RuinationTarget>(ctx.caster)
        .is_none_or(|primary| primary.0 != target)
    {
        return;
    }
    world.entity_mut(ctx.caster).remove::<RuinationTarget>();
    let delay = gauss_ms(395.0, 35.0);
    spell_cast::queued_triggers::schedule_callback(
        world,
        ctx,
        target,
        delay,
        spell_cast::scripts::launch_guldan_meteor,
    );
}

fn gauss_ms(mean: f64, deviation: f64) -> u32 {
    let mut rng = combat::random::thread_rng();
    let uniform: f64 = rng.gen_range(f64::MIN_POSITIVE..1.0);
    let angle = rng.gen_range(0.0..std::f64::consts::TAU);
    let normal = (-2.0 * uniform.ln()).sqrt() * angle.cos();
    (mean + deviation * normal).max(0.0).round() as u32
}
