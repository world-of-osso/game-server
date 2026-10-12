//! TC a352b1fa ArenaTeam.cpp:534-594 and pinned worldserver.conf defaults.
//! Accepted reference formula, not a claim of Blizzard's proprietary Retail MMR.
const ELO_DIVISOR: f32 = 650.0;
const WIN_MODIFIER_LOW: f32 = 48.0;
const WIN_MODIFIER_HIGH: f32 = 24.0;
const LOSS_MODIFIER: f32 = 24.0;
const MATCHMAKER_MODIFIER: f32 = 24.0;

fn win_chance(own: u32, opponent: u32) -> f32 {
    1.0 / (1.0 + (10.0_f32.ln() * (opponent as f32 - own as f32) / ELO_DIVISOR).exp())
}
pub(super) fn rating_delta(own: u32, opponent_mmr: u32, won: bool) -> i32 {
    let chance = win_chance(own, opponent_mmr);
    let modifier = if won {
        if own < 1000 {
            WIN_MODIFIER_LOW
        } else if own < 1300 {
            WIN_MODIFIER_LOW / 2.0 + WIN_MODIFIER_LOW / 2.0 * (1300.0 - own as f32) / 300.0
        } else {
            WIN_MODIFIER_HIGH
        }
    } else {
        LOSS_MODIFIER
    };
    let result = if won {
        modifier * (1.0 - chance)
    } else {
        -modifier * chance
    };
    result.ceil() as i32
}
pub(super) fn matchmaker_delta(own: u32, opponent: u32, won: bool) -> i32 {
    (((if won { 1.0 } else { 0.0 }) - win_chance(own, opponent)) * MATCHMAKER_MODIFIER).ceil()
        as i32
}
