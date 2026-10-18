//! Source126: unresolved child Personal Anchor retains actual-use refusal.
use super::*;

#[test]
fn toyfx126_citizens_real_learning_preserves_unresolved_personal_anchor_refusal() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, UnitAuras};
    const ITEM: u32 = 166247;
    const SPELL: u32 = 287080;
    const CHILD: u32 = 287079;
    const ENTRY: u32 = 148048;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("citizens.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("unsupported toy spell: aura 4")
    );
    h.advance(40);
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| [SPELL, CHILD].contains(&go.spell_id))
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == SPELL)
    );
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| [SPELL, CHILD].contains(&aura.spell_id))
    );
    let world = h.app.world_mut();
    assert!(
        !world
            .query::<(&Npc, &SummonedCreature)>()
            .iter(world)
            .any(|(npc, summon)| npc.template_id == ENTRY && summon.owner == caster)
    );
    assert_eq!(world.get::<Health>(caster), Some(&health));
    assert_eq!(world.get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("unsupported toy spell: aura 4"),
        "unresolved action stays refused rather than becoming an item cooldown"
    );
}
