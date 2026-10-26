//! Literal pinned Feral rows over the shared native driver, for either actor identity.
mod bindings;
mod context;
mod driver;
mod facts;
mod geometry;
mod source;

pub(super) use driver::Rotation;
pub(super) const REPORT_KEY: Option<&str> = Some("feral_rotation");

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    driver::prepare(run)
}
