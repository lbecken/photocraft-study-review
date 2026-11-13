//! The `eol.*` command group (filled by T-018).

use crate::registry::{Registry, RegistryError};

pub(crate) fn register(_registry: &mut Registry) -> Result<(), RegistryError> {
    Ok(())
}
