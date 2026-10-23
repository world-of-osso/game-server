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
const REVENGE: u32 = 6572;
const FREE_REVENGE: u32 = 5302;
const BLOODY_REBUKE: u32 = 1300690;
const RAVAGER: u32 = 228920;
const RAVAGER_TICK: u32 = 156287;

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

fn equip_tier(h: &mut Harness, pieces: usize) {
    // ItemSet2067: real70448 armor identities, not a granted set-bonus aura.
    let items = [
        (EquipmentSlot::Head, 271456),
        (EquipmentSlot::Chest, 271459),
        (EquipmentSlot::Hands, 271457),
        (EquipmentSlot::Legs, 271455),
    ];
    for &(slot, item) in items.iter().take(pieces) {
        h.app
            .world_mut()
            .get_mut::<crate::stats::EquippedItems>(h.caster)
            .unwrap()
            .items
            .push((slot, shared::item_data::ItemDefinitionSource::Retail, item));
        h.app
            .world_mut()
            .get_mut::<crate::stats::EquippedItemModifiers>(h.caster)
            .unwrap()
            .items
            .push((slot, Default::default()));
    }
    h.advance(3);
}

fn landed_revenge(pieces: usize, free: bool) -> Harness {
    for _ in 0..8 {
        let mut h = item_fixture(false);
        equip_tier(&mut h, pieces);
        h.app.world_mut().entity_mut(h.caster).insert(
            crate::class_progression::KnownSpells::fixture(vec![
                23922,
                REVENGE,
                PROTECTION_PASSIVE,
            ]),
        );
        // Actual autos and native Shield Slam energizes fund the paid control.
        // Stop autos before reading the cost boundary; never write a Rage budget.
        h.attack(h.npc);
        h.advance(3);
        for _ in 0..3 {
            h.cast_and_deliver(23922, Some(h.npc));
            h.advance(190);
            assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        }
        h.stop_attack();
        h.advance(35);
        if free {
            let caster = h.caster;
            apply_passive(&mut h, caster, FREE_REVENGE);
        }
        let before = h.power(shared::components::PowerType::Rage);
        assert!(
            before >= 20.0,
            "actual native attacks must fund the20 Rage control"
        );
        h.inbox = Inbox::default();
        h.cast_and_deliver(REVENGE, Some(h.npc));
        h.advance(5);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let after = h.power(shared::components::PowerType::Rage);
        if free {
            assert_eq!(
                after, before,
                "native5302 must really make this Revenge free"
            );
        } else {
            assert!(after < before, "the paid control must spend real Rage");
        }
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(REVENGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            return h;
        }
    }
    panic!("eight native Revenge casts produced no landed damage");
}

#[test]
#[ignore = "RED: equipped four-piece free Revenge has no Bloody Rebuke producer"]
fn protection_producers_item_four_piece_free_revenge_emits_owned_periodic_bloody_rebuke() {
    let mut h = landed_revenge(4, true);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == BLOODY_REBUKE))
        .expect("native equipped four-piece and actual free Revenge must produce Bloody Rebuke");
    assert_eq!(aura.caster, h.caster);
    h.advance(75);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(BLOODY_REBUKE)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0)
    );
}

#[test]
#[ignore = "RED control: Bloody Rebuke requires four pieces and a genuinely free Revenge"]
fn protection_producers_item_three_piece_or_paid_revenge_cannot_grant_bloody_rebuke() {
    for (pieces, free) in [(3, true), (4, false)] {
        let h = landed_revenge(pieces, free);
        assert!(!crate::auras::has_aura(h.app.world(), h.npc, BLOODY_REBUKE));
        assert!(
            !h.inbox
                .log
                .iter()
                .any(|event| event.spell_id == Some(BLOODY_REBUKE))
        );
    }
}

#[test]
#[ignore = "RED: equipped four-piece Ravager ticks have no Bloody Rebuke producer"]
fn protection_producers_item_four_piece_ravager_tick_produces_bloody_rebuke() {
    let mut h = item_fixture(false);
    equip_tier(&mut h, 4);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            RAVAGER,
            PROTECTION_PASSIVE,
        ]));
    h.cast_and_deliver(RAVAGER, Some(h.npc));
    h.advance(90);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(RAVAGER_TICK)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(
        crate::auras::has_aura(h.app.world(), h.npc, BLOODY_REBUKE),
        "a real native Ravager tick with four equipped pieces must produce the bleed"
    );
    h.advance(75);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(BLOODY_REBUKE)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0)
    );
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
