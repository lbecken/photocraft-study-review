//! The `keymap.*` command group and its session state `KeymapState` (filled by T-031).

use crate::registry::{Registry, RegistryError};

pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> {
    Ok(())
}

/// Session state of the `keymap.*` group; its owning task adds the fields.
#[derive(Debug, Default)]
pub(crate) struct KeymapState {}
