//! Native Rune aura395 owns its scripted field; no AT or script hook is fabricated.
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;

const VAULT_RUNE: u32 = 197204;
const RUNE_AURA: u32 = 388065;
const RUNE_PROPERTIES: u32 = 26586;

pub(super) fn fixture(difficulty: u32) -> (Harness, Entity) {
    let (mut h, _telash, _map) = bound_boss_at_difficulty(186737, 2515, 2, difficulty, None);
    let runes = h.creatures_of(VAULT_RUNE);
    assert_eq!(runes.len(), 3, "native Telash group0 prerequisite");
    (h, runes[0])
}

pub(super) fn assert_native_source(h: &Harness) {
    let connection = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("published Rune source database");
    let properties: (u32, u32, f32, f32, String) = connection
        .query_row(
            "SELECT AreaTriggerId,Flags,ShapeData0,ShapeData1,ScriptName \
             FROM tdb_areatrigger_create_properties WHERE Id=?1",
            [RUNE_PROPERTIES],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("native26586 source row prerequisite");
    assert_eq!(
        properties,
        (30743, 4, 7.0, 7.0, "at_telash_greywing_vault_rune".into())
    );
    let effect = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(RUNE_AURA)
        .expect("native388065 spell prerequisite")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!(
        (effect.effect, effect.aura, effect.misc_values[0]),
        (6, 395, RUNE_PROPERTIES as i32)
    );
}

#[test]
fn azure_rune_native_aura395_creates_owned_scripted_sphere() {
    let (mut h, rune) = fixture(205);
    assert_native_source(&h);
    assert!(!h.has_aura(rune, RUNE_AURA));
    crate::spell_cast::creature_cast(h.app.world_mut(), rune, RUNE_AURA, Some(rune), true)
        .expect("actual source Rune casts its full native388065 aura");
    h.advance(0.05);
    assert!(
        h.has_aura(rune, RUNE_AURA),
        "native aura must exist before field RED"
    );
    let fields: Vec<_> = h
        .app
        .world_mut()
        .query::<(&SpellAreaTrigger, &Position, &InMap)>()
        .iter(h.app.world())
        .filter(|(field, _, _)| field.spell_id == RUNE_AURA)
        .map(|(field, position, map)| (field.caster, *position, *map, field.radius))
        .collect();
    assert_eq!(
        fields.len(),
        1,
        "actual ScriptName'd Rune aura must create a field, not an enemy-action substitute"
    );
    let (caster, position, map, radius) = fields[0];
    assert_eq!(caster, rune);
    assert_eq!(h.app.world().get::<Position>(rune), Some(&position));
    assert_eq!(h.app.world().get::<InMap>(rune), Some(&map));
    assert_eq!(radius, 7.0, "source sphere radius, not guessed geometry");
}
