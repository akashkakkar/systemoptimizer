//! File classification sensor (F18) — scans user-approved directories and classifies files by metadata.

mod classify;
mod probe;

pub use classify::classify_by_extension;
pub use probe::FileClassificationProbe;
