//! Replay adapter; native healer decisions and native target mitigation remain authoritative.
use super::healer_intake::Intake;

pub(super) fn replay(_intake: &Intake, _spec: u32) -> Result<serde_json::Value, String> {
    Err("RAW_INTAKE_REPLAY_NOT_IMPLEMENTED".into())
}
