//! The `find.*` command group and its session state `FindState` (filled by T-022).

use crate::registry::{Registry, RegistryError};

pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> {
    Ok(())
}

/// Session state of the `find.*` group; its owning task adds the fields.
#[derive(Debug, Default)]
pub(crate) struct FindState {}
