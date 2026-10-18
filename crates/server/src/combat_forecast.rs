//! Current generic-creature melee in tank-health units, before shields/redirect credit.
//! Scripted action schedules are deliberately unsupported, never guessed.
use super::*;

#[derive(Debug)]
pub(crate) struct MeleeEstimate {
    pub expected_hit: f64,
    pub largest_hit: f64,
    pub interval_secs: f64,
    pub next_swing_secs: f64,
    pub remaining_health: f64,
    pub valid_until: Option<f64>,
    pub level: u8,
    pub rank: u8,
}

struct SwingProjection<'a> {
    attacker: Attacker<'a>,
    source_scaling: UnitScaling<'a>,
    target_scaling: UnitScaling<'a>,
    target: Entity,
    chances: melee::MeleeChances,
    target_mods: Option<&'a AuraStatMods>,
    armor: f32,
    versus: f32,
    blocked_pct: f32,
    level_difference: i32,
}

pub(crate) fn read_generic_creature(
    world: &World,
    enemy: Entity,
    tank: Entity,
) -> Result<MeleeEstimate, String> {
    let template = read_generic_template(world, enemy, tank)?;
    let projection = read_swing_projection(world, enemy, tank)?;
    read_estimate(world, enemy, template.rank, &projection)
}

fn read_estimate(
    world: &World,
    enemy: Entity,
    rank: u8,
    projection: &SwingProjection,
) -> Result<MeleeEstimate, String> {
    let (min, max) = projection
        .source_scaling
        .swing_damage_range(AUTO_ATTACK_INTERVAL)
        .unwrap_or((BASE_DAMAGE, BASE_DAMAGE));
    let expected_hit = expected_damage(world, projection, (min + max) * 0.5);
    let largest_hit = largest_damage(world, projection, max);
    let tank_level_health = world.get::<Health>(enemy).unwrap().current
        * projection
            .source_scaling
            .health_multiplier(&projection.target_scaling);
    let estimate = MeleeEstimate {
        expected_hit,
        largest_hit,
        interval_secs: f64::from(projection.attacker.swing_time(&WeaponDamage::UNARMED)),
        next_swing_secs: world
            .get::<AutoAttack>(enemy)
            .map_or(0.0, |attack| f64::from(attack.timer.max(0.0))),
        remaining_health: f64::from(tank_level_health),
        valid_until: read_stat_deadline(world, projection),
        level: world
            .get::<shared::components::UnitLevel>(enemy)
            .ok_or("source creature lacks native level")?
            .0,
        rank,
    };
    validate_estimate(estimate)
}

fn read_generic_template(
    world: &World,
    enemy: Entity,
    tank: Entity,
) -> Result<&crate::world_data::CreatureTemplate, String> {
    validate_candidate(world, enemy, tank)?;
    let template = read_source_template(world, enemy)?;
    validate_script_catalog(world, enemy, template)?;
    Ok(template)
}

fn validate_candidate(world: &World, enemy: Entity, tank: Entity) -> Result<(), String> {
    let map = world
        .get::<InMap>(enemy)
        .ok_or("source creature lacks map")?;
    if world.get::<InMap>(tank) != Some(map)
        || !crate::spell_cast::targets::is_alive(world, enemy)
        || !crate::spell_cast::targets::is_alive(world, tank)
    {
        return Err("forecast requires living units in the same copy".into());
    }
    if world.get::<CharacterStats>(enemy).is_some()
        || world
            .get::<crate::zones::CreatureOrigin>(enemy)
            .is_none_or(|origin| origin.guid == 0)
    {
        return Err("forecast requires an ordinary source-spawn creature".into());
    }
    Ok(())
}

fn read_source_template(
    world: &World,
    enemy: Entity,
) -> Result<&crate::world_data::CreatureTemplate, String> {
    let entry = world
        .get::<Npc>(enemy)
        .ok_or("source creature lacks template identity")?
        .template_id;
    let difficulty = world
        .get::<crate::zones::CreatureDifficulty>(enemy)
        .map_or(0, |difficulty| difficulty.0);
    let data = world
        .get_resource::<crate::world_data::WorldData>()
        .ok_or("source templates unavailable")?;
    let template = data
        .template_for(entry, difficulty)
        .ok_or("source difficulty template unavailable")?;
    Ok(template)
}

fn validate_script_catalog(
    world: &World,
    enemy: Entity,
    template: &crate::world_data::CreatureTemplate,
) -> Result<(), String> {
    let entry = template.entry;
    let scripts = world
        .get_resource::<crate::scripted_ai::smart_ai::SmartScripts>()
        .ok_or("source AI catalog unavailable")?;
    let scripted = !template.script_name.is_empty()
        || scripts.0.contains_key(&entry)
        || world
            .get::<crate::scripted_ai::CreatureScriptName>(enemy)
            .is_some();
    if scripted {
        return Err(format!(
            "scripted source{entry} requires an explicit pull decision; no numerical forecast"
        ));
    }
    Ok(())
}

fn read_attacker(world: &World, enemy: Entity) -> Result<Attacker<'_>, String> {
    Ok(Attacker {
        entity: enemy,
        position: world
            .get::<Position>(enemy)
            .ok_or("source creature lacks position")?,
        mods: world.get::<AuraStatMods>(enemy),
        stats: None,
        haste_pct: world
            .get::<crate::stats::StatModifiers>(enemy)
            .map_or(0.0, |mods| mods.haste_pct),
        creature: true,
        spirit_wolf: None,
    })
}

fn read_swing_projection(
    world: &World,
    enemy: Entity,
    tank: Entity,
) -> Result<SwingProjection<'_>, String> {
    let attacker = read_attacker(world, enemy)?;
    let source_scaling = UnitScaling::of(world, enemy);
    let target_scaling = UnitScaling::of(world, tank);
    let level_difference = i32::from(target_scaling.level_against(&source_scaling))
        - i32::from(source_scaling.level_against(&target_scaling));
    let target_auras = world.get::<crate::auras::ActiveAuras>(tank);
    let sitting = world
        .get::<shared::components::PlayerStandState>(tank)
        .is_some_and(|state| !state.0.is_stand());
    let victim = swing_victim(
        world.get::<Npc>(tank).is_some(),
        world.get::<CharacterStats>(tank),
        level_difference,
        sitting,
    );
    let avoids = can_avoid((
        world.get::<shared::casting::CastState>(tank).is_some(),
        target_auras,
    ));
    let flags = world.get::<crate::zones::CreatureFlagsExtra>(tank);
    let chances = read_outcome_chances(&attacker, victim, flags, avoids, target_auras);
    let armor = read_armor(world, tank, &source_scaling, &target_scaling);
    let blocked_pct = blocked_pct(
        victim,
        &source_scaling,
        &target_scaling,
        world.get_resource::<ArmorConstants>(),
    );
    let versus = read_damage_versus(world, attacker.mods, tank)?;
    Ok(SwingProjection {
        attacker,
        source_scaling,
        target_scaling,
        target: tank,
        chances,
        target_mods: world.get::<AuraStatMods>(tank),
        armor,
        versus,
        blocked_pct,
        level_difference,
    })
}

fn read_armor(world: &World, tank: Entity, source: &UnitScaling, target: &UnitScaling) -> f32 {
    target_armor(
        (
            world.get::<CreatureArmor>(tank),
            world.get::<CharacterStats>(tank),
            world.get::<crate::stats::StatModifiers>(tank),
            world.get::<crate::hunter_pet::HunterPet>(tank),
            world.get::<crate::spell_cast::summon::SummonedCreature>(tank),
        ),
        target,
        source,
    )
}

fn read_damage_versus(
    world: &World,
    mods: Option<&AuraStatMods>,
    tank: Entity,
) -> Result<f32, String> {
    let Some(mods) = mods.filter(|mods| mods.has_damage_done_versus()) else {
        return Ok(1.0);
    };
    let kind = crate::unit_creature_type::CreatureTypes::from_world(world).resolve(
        world.get::<Player>(tank),
        world.get::<Npc>(tank),
        world.get::<crate::auras::ActiveAuras>(tank),
    )?;
    Ok(mods.damage_done_versus(crate::unit_creature_type::mask(kind)))
}

fn armored_damage(world: &World, projection: &SwingProjection, base: f32) -> f32 {
    let modified = apply_swing_multipliers(
        base,
        projection.attacker.mods,
        projection.target_mods,
        projection.versus,
    );
    armor_mitigated(
        modified,
        projection.armor,
        &projection.source_scaling,
        &projection.target_scaling,
        world.get_resource::<ArmorConstants>(),
    )
}

fn outcome_health_loss(projection: &SwingProjection, damage: f32, outcome: MeleeOutcome) -> f64 {
    let (damage, _) = melee::outcome_damage(
        damage,
        outcome,
        projection.level_difference,
        projection.blocked_pct,
    );
    let critical = if outcome == MeleeOutcome::Crit {
        projection
            .attacker
            .mods
            .map_or(1.0, |mods| mods.critical_damage(1))
    } else {
        1.0
    };
    let dealt = crate::level_scaling::damage_dealt(
        &projection.source_scaling,
        &projection.target_scaling,
        damage * critical,
    );
    f64::from(crate::level_scaling::health_loss(
        &projection.source_scaling,
        &projection.target_scaling,
        dealt,
    ))
}

fn expected_damage(world: &World, projection: &SwingProjection, base: f32) -> f64 {
    let damage = armored_damage(world, projection, base);
    melee::outcome_weights(&projection.chances)
        .into_iter()
        .map(|(outcome, weight)| outcome_health_loss(projection, damage, outcome) * weight)
        .sum()
}

fn largest_damage(world: &World, projection: &SwingProjection, base: f32) -> f64 {
    let damage = armored_damage(world, projection, base);
    melee::outcome_weights(&projection.chances)
        .into_iter()
        .map(|(outcome, _)| outcome_health_loss(projection, damage, outcome))
        .fold(0.0, f64::max)
}

fn read_stat_deadline(world: &World, projection: &SwingProjection) -> Option<f64> {
    [projection.attacker.entity, projection.target]
        .into_iter()
        .filter_map(|unit| {
            let auras = world.get::<crate::auras::ActiveAuras>(unit)?;
            crate::auras::forecast::first_melee_stat_expiry(unit, auras)
        })
        .min_by(f64::total_cmp)
}

fn validate_estimate(estimate: MeleeEstimate) -> Result<MeleeEstimate, String> {
    let values = [
        estimate.expected_hit,
        estimate.largest_hit,
        estimate.interval_secs,
        estimate.next_swing_secs,
        estimate.remaining_health,
    ];
    let valid = values
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
        && estimate.interval_secs > 0.0
        && estimate.remaining_health > 0.0;
    if valid {
        Ok(estimate)
    } else {
        Err("native creature forecast contains invalid stats/timing".into())
    }
}
