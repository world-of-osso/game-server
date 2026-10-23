//! Equipped70448 item producers; native item/set/role source ingress precedes scripts.
use super::{DEVASTATE, Harness, Inbox, apply_passive, fixture};
use shared::protocol::{CombatLogKind, EquipmentSlot};
use std::path::PathBuf;
use std::sync::OnceLock;

const GUILLOTINE_ITEM: u32 = 270173;
const GUILLOTINE_DRIVER: u32 = 1291728;
const GUILLOTINE_DAMAGE: u32 = 1306604;
const PERFECTED_GUILLOTINE: u32 = 1306624;
const PROTECTION_PASSIVE: u32 = 137048;

fn native_catalog_connection() -> rusqlite::Connection {
    static DIRECTORY: OnceLock<tempfile::TempDir> = OnceLock::new();
    let directory = DIRECTORY.get_or_init(|| {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/protection-item-fixtures");
        std::fs::create_dir_all(&root).unwrap();
        let directory = tempfile::tempdir_in(root).unwrap();
        let path = directory.path().join("producer.db");
        std::fs::File::create(&path).unwrap();
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let output = std::process::Command::new("python3")
            .arg(repo.join("scripts/item_instance_data.py"))
            .arg("--db2-dir")
            .arg(repo.join("data/db2/12.1.5.70448"))
            .arg("--db")
            .arg(&path)
            .output()
            .expect("import native equipped-producer catalogs into an isolated database");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        directory
    });
    rusqlite::Connection::open_with_flags(
        directory.path().join("producer.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
}

fn item_fixture(equipped: bool) -> Harness {
    let mut h = fixture(false);
    let conn = native_catalog_connection();
    h.app.insert_resource(crate::stats::NativeItemInstances {
        bonuses: server::item_instance::load::load_bonus_tables(&conn).unwrap(),
        enchants: server::item_instance::load::load_enchantment_tables(&conn).unwrap(),
        effects: server::item_instance::load::load_item_effect_tables(&conn).unwrap(),
    });
    h.app.world_mut().entity_mut(h.caster).insert((
        shared::components::UnitLevel(90),
        crate::class_progression::ActiveSpec(73),
        crate::class_progression::KnownSpells::fixture(vec![DEVASTATE, PROTECTION_PASSIVE]),
    ));
    h.app
        .world_mut()
        .get_mut::<shared::components::Level>(h.caster)
        .unwrap()
        .0 = 90;
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        });
    let caster = h.caster;
    apply_passive(&mut h, caster, PROTECTION_PASSIVE);
    let mut modifiers = crate::stats::EquippedItemModifiers::default();
    if equipped {
        h.app
            .world_mut()
            .get_mut::<crate::stats::EquippedItems>(caster)
            .unwrap()
            .items
            .push((
                EquipmentSlot::Trinket1,
                shared::item_data::ItemDefinitionSource::Retail,
                GUILLOTINE_ITEM,
            ));
        modifiers
            .items
            .push((EquipmentSlot::Trinket1, Default::default()));
    }
    h.app.world_mut().entity_mut(caster).insert(modifiers);
    h.select(Some(h.npc));
    h.advance(3);
    assert_eq!(
        crate::auras::has_aura(h.app.world(), caster, GUILLOTINE_DRIVER),
        equipped
    );
    h.inbox = Inbox::default();
    h
}

fn devastate(h: &mut Harness) {
    h.cast_and_deliver(DEVASTATE, Some(h.npc));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
#[ignore = "RED: equipped native Guillotine has no damage producer"]
fn protection_producers_item_equipped_guillotine_emits_base_not_catalog_perfected_variant() {
    let mut h = item_fixture(true);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == GUILLOTINE_DRIVER)
        .unwrap();
    assert_eq!(aura.cast_item_level, Some(219));
    assert!(
        aura.effects
            .iter()
            .any(|effect| effect.index == 0 && effect.amount > 0.0),
        "real native item-level scaling must resolve the driver before testing its producer"
    );
    for _ in 0..60 {
        devastate(&mut h);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(GUILLOTINE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            assert!(
                !h.inbox
                    .log
                    .iter()
                    .any(|event| event.spell_id == Some(PERFECTED_GUILLOTINE)),
                "one equipped item cannot grant the two-piece perfected variant"
            );
            return;
        }
    }
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(DEVASTATE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    panic!(
        "actual equipped Guillotine driver never emitted its native damage child during60 attacks"
    );
}

#[test]
#[ignore = "RED control: qualify catalog absence and native unequip lifecycle with Guillotine"]
fn protection_producers_item_guillotine_catalog_and_unequipped_driver_do_not_proc() {
    for remove_equipped in [false, true] {
        let mut h = item_fixture(remove_equipped);
        if remove_equipped {
            h.app
                .world_mut()
                .get_mut::<crate::stats::EquippedItems>(h.caster)
                .unwrap()
                .items
                .retain(|(slot, _, _)| *slot != EquipmentSlot::Trinket1);
            h.app
                .world_mut()
                .get_mut::<crate::stats::EquippedItemModifiers>(h.caster)
                .unwrap()
                .items
                .clear();
            h.advance(3);
            assert!(!crate::auras::has_aura(
                h.app.world(),
                h.caster,
                GUILLOTINE_DRIVER
            ));
        }
        for _ in 0..60 {
            devastate(&mut h);
        }
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.spell_id == Some(DEVASTATE)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0)
        );
        assert!(
            !h.inbox.log.iter().any(|event| matches!(
                event.spell_id,
                Some(GUILLOTINE_DAMAGE | PERFECTED_GUILLOTINE)
            )),
            "catalog presence and removed equipment cannot grant Guillotine"
        );
    }
}
