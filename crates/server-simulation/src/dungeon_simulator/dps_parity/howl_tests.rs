//! SIMC_BM_HOWL_OF_THE_PACK_LEADER_CYCLE: actual native actors, never fake readiness.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

const HOWL: u32 = 471876;
const HOWL_COOLDOWN: u32 = 471877;
const WYVERN_READY: u32 = 471878;
const WYVERN_SUMMON: u32 = 1222271;
const WYVERN_CRY: u32 = 471881;
const WYVERN_ENTRY: u32 = 234170;
const CALL_PET: u32 = 883;
const KILL_COMMAND: u32 = 34026;

fn prepare_howl_player() -> Run {
    let (profile, traits) = read_reference(253).unwrap();
    let run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, HOWL));
    let data = run.app.world().resource::<crate::world_data::WorldData>();
    assert!(
        data.templates.contains_key(&WYVERN_ENTRY),
        "normal authentic summon closure is a prerequisite, not gameplay RED"
    );
    run
}

fn advance_seconds(run: &mut Run, seconds: f64) {
    let end = crate::spell_cast::now(run.app.world()) + seconds;
    while crate::spell_cast::now(run.app.world()) + 1e-6 < end {
        run.app.update();
    }
}

fn owned_wyverns(run: &mut Run) -> Vec<Entity> {
    let owner = run.actor;
    let world = run.app.world_mut();
    let mut query = world.query::<(Entity, &Npc, &SummonedCreature)>();
    query
        .iter(world)
        .filter(|(_, npc, summon)| {
            npc.template_id == WYVERN_ENTRY
                && summon.owner == owner
                && summon.spell_id == WYVERN_SUMMON
        })
        .map(|(entity, _, _)| entity)
        .collect()
}

fn native_duration(run: &Run, spell: u32) -> f64 {
    let info = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap();
    f64::from(info.misc.as_ref().unwrap().duration.unwrap().duration_ms) / 1000.0
}

fn spawn_native_wyvern(run: &mut Run) -> Entity {
    let owner = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, WYVERN_SUMMON, Some(owner), true)
        .expect("native summon child admission includes authored destination geometry");
    run.app.update();
    let summons = owned_wyverns(run);
    assert_eq!(
        summons.len(),
        1,
        "real effect28 must create one authentic owned native entity"
    );
    summons[0]
}

fn cry_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == WYVERN_CRY && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "native Howl real summon closure/lifetime control; prepared253/current world required"]
fn dps_parity_howl_native_child_uses_authentic_entity_and_native_lifetime() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_howl_player();
    let duration = native_duration(&run, WYVERN_SUMMON);
    let actor = spawn_native_wyvern(&mut run);
    let expiry = run
        .app
        .world()
        .get::<SummonedCreature>(actor)
        .unwrap()
        .expires_at
        .unwrap();
    assert!(expiry > crate::spell_cast::now(run.app.world()));
    advance_seconds(&mut run, duration + 0.1);
    assert!(
        run.app.world().get_entity(actor).is_err(),
        "actual native duration must retire the entity"
    );
}

#[test]
#[ignore = "native Howl Cry earned from actual summon; not an apply-aura fixture"]
fn dps_parity_howl_native_real_wyvern_earns_db2_cry_stacks() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_howl_player();
    assert_eq!(cry_stacks(&run), 0);
    let expected = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(HOWL)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .base_points;
    let actor = spawn_native_wyvern(&mut run);
    assert!(run.app.world().get_entity(actor).is_ok());
    assert_eq!(
        cry_stacks(&run),
        expected as u8,
        "real selected Howl summon earns native root effect2 stack count"
    );
}

#[test]
#[ignore = "native actual summon without selected Howl must not grant Cry"]
fn dps_parity_howl_native_removed_passive_keeps_real_summon_without_cry() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_howl_player();
    let owner = run.actor;
    let removed = crate::auras::remove_auras(
        run.app.world_mut(),
        owner,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == HOWL,
    );
    assert_eq!(removed.len(), 1);
    spawn_native_wyvern(&mut run);
    assert_eq!(cry_stacks(&run), 0);
}

#[test]
#[ignore = "native combat clock to ready to ordinary KC and actual summoned Wyvern"]
fn dps_parity_howl_native_combat_readiness_is_consumed_by_real_kill_command() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_howl_player();
    let owner = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, CALL_PET, Some(owner), false)
        .expect("actual native Player Call Pet, no fabricated HunterPet");
    advance_seconds(&mut run, 2.0);
    assert!(crate::hunter_pet::summoned_pet(run.app.world(), owner).is_some());
    engage(&mut run);
    let cooldown = native_duration(&run, HOWL_COOLDOWN);
    advance_seconds(&mut run, cooldown + 0.2);
    assert!(
        crate::auras::has_aura(run.app.world(), owner, WYVERN_READY),
        "native combat entry and elapsed DB2 cooldown must earn first ready aura"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        owner,
        KILL_COMMAND,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary native KC with actual persistent pet");
    advance_seconds(&mut run, 0.2);
    assert!(
        !crate::auras::has_aura(run.app.world(), owner, WYVERN_READY),
        "successful real summon consumes readiness exactly once"
    );
    assert_eq!(owned_wyverns(&mut run).len(), 1);
    assert!(cry_stacks(&run) > 0, "Cry follows the real summoned actor");
    assert!(
        crate::auras::has_aura(run.app.world(), owner, HOWL_COOLDOWN),
        "consumption rearms native cooldown"
    );
}
