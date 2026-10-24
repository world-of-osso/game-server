//! Qualify real Retail bag item use before adding reference-profile precombat orchestration.
use super::{Run, prepare};
use crate::auras::ActiveAuras;
use crate::persistence::GameDatabase;
use serde_json::json;
use shared::components::Player;
use shared::protocol::UseItem;

const SHATTERED_SUN_ITEM: u32 = 241326;
const SHATTERED_SUN_SPELL: u32 = 1235111;
const MAGISTERS_ITEM: u32 = 241322;
const MAGISTERS_SPELL: u32 = 1235108;
const VOID_TOUCHED_ITEM: u32 = 259085;
const VOID_TOUCHED_SPELL: u32 = 1264426;

fn install_character_inventory(run: &mut Run) -> u64 {
    let player = run
        .app
        .world()
        .get::<Player>(run.actor)
        .expect("native Player");
    let db = crate::persistence::temp_database();
    let (character, _) =
        crate::persistence::create_character(&db, 1, "NativeConsume", player.race, player.class)
            .expect("real private character record");
    run.app.world_mut().insert_resource(GameDatabase::new(db));
    run.app
        .world_mut()
        .init_resource::<crate::gold::GoldUpdateQueue>();
    character
}

fn prepare_inventory(spec: u32) -> (Run, u64) {
    let mut run = prepare(spec, 1).expect("actual reference gear, race and talents");
    let character = install_character_inventory(&mut run);
    let path = std::env::var_os("DPS_PARITY_WORLD_DB").expect("private native world database");
    let source =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("read native item-use catalogs");
    let vendors =
        crate::merchant::load_vendor_catalog(&source).expect("native item grant metadata");
    let toys = crate::toy_catalog::load(&source).expect("native item-use toy classification");
    run.app.world_mut().insert_resource(vendors);
    run.app.world_mut().insert_resource(toys);
    (run, character)
}

fn grant_native_item(run: &Run, character: u64, item: u32) {
    let world = run.app.world();
    let db = world.resource::<GameDatabase>();
    crate::inventory::grant_item_with_origin(
        &db.db,
        world.resource::<crate::item_templates::ItemTemplates>(),
        world.resource::<crate::merchant::VendorCatalog>(),
        character,
        (item, 2),
        None,
    )
    .expect("grant actual Retail inventory item with native name/quality/price");
}

fn count_item(run: &Run, character: u64, item: u32) -> u32 {
    crate::persistence::load_inventory(&run.app.world().resource::<GameDatabase>().db, character)
        .items
        .iter()
        .filter(|entry| entry.item_id == item)
        .map(|entry| entry.stack_count)
        .sum()
}

fn read_item_location(run: &Run, character: u64, item: u32) -> shared::protocol::ItemLocation {
    let world = run.app.world();
    let model = crate::inventory::load_model(
        &world.resource::<GameDatabase>().db,
        world.resource::<crate::item_templates::ItemTemplates>(),
        character,
    )
    .expect("real native inventory model");
    model
        .view()
        .into_iter()
        .find(|(_, entry)| entry.item_id == item)
        .expect("granted native bag item")
        .0
}

fn use_native_item(run: &mut Run, character: u64, item: u32, spell: u32) {
    let location = read_item_location(run, character, item);
    crate::inventory::use_item(
        run.app.world_mut(),
        character,
        run.actor,
        &UseItem {
            location,
            target: Some(run.actor.to_bits()),
        },
    )
    .expect("ordinary native item-use entry");
    for _ in 0..200 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), run.actor, spell) {
            break;
        }
    }
    // Let the existing native stat producer reconcile the completed item spell.
    for _ in 0..4 {
        run.app.update();
    }
}

fn aura_values(run: &Run, spell: u32) -> serde_json::Value {
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell);
    json!(aura.map(
        |aura| json!({"spell": aura.spell_id, "duration_ms": aura.duration_ms,
        "effects": aura.effects.iter().map(|effect| json!({"index":effect.index,
            "amount":effect.amount})).collect::<Vec<_>>()})
    ))
}

fn write_outcome(
    run: &Run,
    character: u64,
    item: (u32, u32),
    before: serde_json::Value,
) -> serde_json::Value {
    let (item, spell) = item;
    let trace = json!({"spec":run.profile.spec_id,"item":item,"spell":spell,"before":before,
        "after":super::report::stats(run.app.world(), run.actor),"aura":aura_values(run,spell),
        "remaining_items":count_item(run,character,item)});
    let path = super::inputs::directory()
        .unwrap()
        .join(format!("consumable-{}-{item}.json", run.profile.spec_id));
    std::fs::write(path, serde_json::to_vec_pretty(&trace).unwrap()).unwrap();
    trace
}

fn qualify_flask(spec: u32, item: u32, spell: u32, rating: &str) {
    let (mut run, character) = prepare_inventory(spec);
    grant_native_item(&run, character, item);
    let before = super::report::stats(run.app.world(), run.actor);
    assert_eq!(count_item(&run, character, item), 2);
    use_native_item(&mut run, character, item, spell);
    let trace = write_outcome(&run, character, (item, spell), before);
    assert!(
        !trace["aura"].is_null(),
        "real item spell must apply native aura: {trace}"
    );
    assert_eq!(
        trace["remaining_items"], 1,
        "native expendable item consumes exactly one"
    );
    let delta = trace["after"]["ratings"][rating].as_f64().unwrap()
        - trace["before"]["ratings"][rating].as_f64().unwrap();
    assert_eq!(
        delta, 165.0,
        "native tier2 ilvl295 flask amount, not a stat grant: {trace}"
    );
}

fn assert_rune_outcome(trace: &serde_json::Value) {
    let (before, after, aura) = (&trace["before"], &trace["after"], &trace["aura"]);
    assert!(
        !aura.is_null(),
        "actual rune item must apply its native spell: {trace}"
    );
    assert_eq!(trace["remaining_items"], 1);
    assert_eq!(
        aura["effects"][0]["amount"].as_f64().unwrap(),
        25.0,
        "native level90 raw amount"
    );
    assert!(
        after["attribute"]["agility"].as_f64().unwrap()
            > before["attribute"]["agility"].as_f64().unwrap(),
        "highest native primary must change: {trace}"
    );
    assert_eq!(
        after["attribute"]["strength"],
        before["attribute"]["strength"]
    );
    assert_eq!(
        after["attribute"]["intellect"],
        before["attribute"]["intellect"]
    );
}

fn qualify_rune(spec: u32) {
    let (mut run, character) = prepare_inventory(spec);
    grant_native_item(&run, character, VOID_TOUCHED_ITEM);
    let before = super::report::stats(run.app.world(), run.actor);
    use_native_item(&mut run, character, VOID_TOUCHED_ITEM, VOID_TOUCHED_SPELL);
    let trace = write_outcome(
        &run,
        character,
        (VOID_TOUCHED_ITEM, VOID_TOUCHED_SPELL),
        before,
    );
    assert_rune_outcome(&trace);
}

#[test]
#[ignore = "actual Retail bag/character/item-spell path; private reference inputs required"]
fn native_consumable_havoc_shattered_sun_uses_real_inventory_and_native_rating() {
    let _seed = crate::random::seed_scope(70_448);
    qualify_flask(577, SHATTERED_SUN_ITEM, SHATTERED_SUN_SPELL, "crit");
}

#[test]
#[ignore = "actual Retail bag/character/item-spell path; private reference inputs required"]
fn native_consumable_feral_magisters_uses_real_inventory_and_native_rating() {
    let _seed = crate::random::seed_scope(70_448);
    qualify_flask(103, MAGISTERS_ITEM, MAGISTERS_SPELL, "mastery");
}

#[test]
#[ignore = "actual Retail rune item and highest-primary producer; private inputs required"]
fn native_consumable_havoc_void_touched_scales_and_selects_highest_primary() {
    let _seed = crate::random::seed_scope(70_448);
    qualify_rune(577);
}

#[test]
#[ignore = "actual Retail rune item and WW source control; private inputs required"]
fn native_consumable_windwalker_void_touched_scales_and_selects_highest_primary() {
    let _seed = crate::random::seed_scope(70_448);
    qualify_rune(269);
}
