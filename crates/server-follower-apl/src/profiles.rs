//! Pinned SimC profiles the follower APLs read: action lists, consumables and options.
//! DPS and most tank profiles come from `scripts/generate_follower_apl_profiles.py`;
//! healers and Guardian use the same tracked profiles as their parity fixtures.
use server::followers::parity_profile::Profile;

pub fn profile(spec: u32) -> Result<Profile, String> {
    let text = match spec {
        62 => include_str!("../profiles/62.profile.json"),
        63 => include_str!("../profiles/63.profile.json"),
        64 => include_str!("../profiles/64.profile.json"),
        65 => include_str!("../../../scripts/reference/healer-tank-top-logs/65.profile.json"),
        66 => include_str!("../profiles/66.profile.json"),
        70 => include_str!("../profiles/70.profile.json"),
        71 => include_str!("../profiles/71.profile.json"),
        72 => include_str!("../profiles/72.profile.json"),
        73 => include_str!("../profiles/73.profile.json"),
        102 => include_str!("../profiles/102.profile.json"),
        103 => include_str!("../profiles/103.profile.json"),
        104 => include_str!("../../../scripts/reference/healer-tank-top-logs/104.profile.json"),
        105 => include_str!("../../../scripts/reference/healer-tank-top-logs/105.profile.json"),
        250 => include_str!("../profiles/250.profile.json"),
        251 => include_str!("../profiles/251.profile.json"),
        252 => include_str!("../profiles/252.profile.json"),
        253 => include_str!("../profiles/253.profile.json"),
        254 => include_str!("../profiles/254.profile.json"),
        255 => include_str!("../profiles/255.profile.json"),
        256 => include_str!("../../../scripts/reference/healer-tank-top-logs/256.profile.json"),
        257 => include_str!("../../../scripts/reference/healer-tank-top-logs/257.profile.json"),
        258 => include_str!("../profiles/258.profile.json"),
        259 => include_str!("../profiles/259.profile.json"),
        260 => include_str!("../profiles/260.profile.json"),
        261 => include_str!("../profiles/261.profile.json"),
        262 => include_str!("../profiles/262.profile.json"),
        263 => include_str!("../profiles/263.profile.json"),
        264 => include_str!("../../../scripts/reference/healer-tank-top-logs/264.profile.json"),
        265 => include_str!("../profiles/265.profile.json"),
        266 => include_str!("../profiles/266.profile.json"),
        267 => include_str!("../profiles/267.profile.json"),
        268 => include_str!("../profiles/268.profile.json"),
        269 => include_str!("../profiles/269.profile.json"),
        270 => include_str!("../../../scripts/reference/healer-tank-top-logs/270.profile.json"),
        577 => include_str!("../profiles/577.profile.json"),
        581 => include_str!("../profiles/581.profile.json"),
        1467 => include_str!("../profiles/1467.profile.json"),
        1468 => include_str!("../../../scripts/reference/healer-tank-top-logs/1468.profile.json"),
        1480 => include_str!("../profiles/1480.profile.json"),
        spec => return Err(format!("spec{spec} has no tracked APL profile")),
    };
    serde_json::from_str(text).map_err(|error| format!("spec{spec} APL profile: {error}"))
}
