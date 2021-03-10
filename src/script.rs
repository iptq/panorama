//! Everything dealing with scripting

use anyhow::Result;
use gluon::{import::add_extern_module, ThreadExt};

/// Creates a VM for running scripts
pub async fn create_script_vm() -> Result<()> {
    let vm = gluon::new_vm_async().await;

    Ok(())
}
