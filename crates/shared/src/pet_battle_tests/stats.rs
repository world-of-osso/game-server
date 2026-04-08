use super::*;

#[test]
fn level_multiplier_at_level_1() {
    let stats = compute_stats(&test_species(), 1, PetQuality::Common);
    assert_eq!(stats.health, 105);
    assert_eq!(stats.power, 20);
}

#[test]
fn level_multiplier_at_level_25() {
    let stats = compute_stats(&test_species(), 25, PetQuality::Common);
    assert_eq!(stats.health, 625);
    assert_eq!(stats.power, 100);
    assert_eq!(stats.speed, 100);
}

#[test]
fn level_multiplier_mid_level() {
    let stats = compute_stats(&test_species(), 13, PetQuality::Common);
    assert_eq!(stats.health, 365);
}

#[test]
fn stats_level1_common() {
    let stats = compute_stats(&test_species(), 1, PetQuality::Common);
    assert_eq!(stats.health, 105);
    assert_eq!(stats.power, 20);
    assert_eq!(stats.speed, 20);
}

#[test]
fn stats_level25_common() {
    let stats = compute_stats(&test_species(), 25, PetQuality::Common);
    assert_eq!(stats.health, 625);
    assert_eq!(stats.power, 100);
    assert_eq!(stats.speed, 100);
}

#[test]
fn stats_rare_better_than_common() {
    let common = compute_stats(&test_species(), 10, PetQuality::Common);
    let rare = compute_stats(&test_species(), 10, PetQuality::Rare);
    assert!(rare.health > common.health);
    assert!(rare.power > common.power);
    assert!(rare.speed > common.speed);
}

#[test]
fn stats_higher_level_better() {
    let low = compute_stats(&test_species(), 5, PetQuality::Common);
    let high = compute_stats(&test_species(), 20, PetQuality::Common);
    assert!(high.health > low.health);
    assert!(high.power > low.power);
    assert!(high.speed > low.speed);
}

#[test]
fn stats_quality_ordering() {
    let species = test_species();
    let level = 10;
    let poor = compute_stats(&species, level, PetQuality::Poor);
    let common = compute_stats(&species, level, PetQuality::Common);
    let uncommon = compute_stats(&species, level, PetQuality::Uncommon);
    let rare = compute_stats(&species, level, PetQuality::Rare);
    let epic = compute_stats(&species, level, PetQuality::Epic);
    let legendary = compute_stats(&species, level, PetQuality::Legendary);
    assert!(poor.power < common.power);
    assert!(common.power < uncommon.power);
    assert!(uncommon.power < rare.power);
    assert!(rare.power < epic.power);
    assert!(epic.power < legendary.power);
}

#[test]
fn stats_poor_quality_penalty() {
    let common = compute_stats(&test_species(), 10, PetQuality::Common);
    let poor = compute_stats(&test_species(), 10, PetQuality::Poor);
    assert!(poor.power < common.power);
    assert!(poor.speed < common.speed);
}

#[test]
fn stats_level25_rare_concrete() {
    let stats = compute_stats(&test_species(), 25, PetQuality::Rare);
    assert_eq!(stats.health, 725);
    assert_eq!(stats.power, 120);
    assert_eq!(stats.speed, 120);
}
