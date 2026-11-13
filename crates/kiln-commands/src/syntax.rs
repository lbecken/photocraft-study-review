//! The `syntax.*` command group and its session state `SyntaxState` (filled by T-020).

use crate::registry::{Registry, RegistryError};

pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> {
    Ok(())
}

/// Session state of the `syntax.*` group; its owning task adds the fields.
#[derive(Debug, Default)]
pub(crate) struct SyntaxState {}
