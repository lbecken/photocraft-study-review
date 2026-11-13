//! The `diag.*` command group and its session state `DiagState` (filled by T-024).

use crate::registry::{Registry, RegistryError};

pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> {
    Ok(())
}

/// Session state of the `diag.*` group; its owning task adds the fields.
#[derive(Debug, Default)]
pub(crate) struct DiagState {}
