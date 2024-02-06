use std::error::Error;
use std::future::pending;
use std::string::ToString;

use tracing::{debug, info};
use zbus::ConnectionBuilder;

use updates::server::Server;

const INTERFACE_NAME: &str = "dev.rlxos.updates";
const OBJECT_PATH: &str = "/dev/rlxos/updates";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    setup_namespaces()?;

    let server = Server::new()?;

    let _conn = ConnectionBuilder::system()?
        .name(INTERFACE_NAME)?
        .serve_at(OBJECT_PATH, server)?
        .build()
        .await?;

    info!("listening at {} {}", INTERFACE_NAME, OBJECT_PATH);
    pending::<()>().await;
    Ok(())
}

pub fn setup_namespaces() -> Result<(), updates::Error> {
    debug!("Checking permissions");
    if nix::unistd::getegid().as_raw() != 0 {
        return Err(updates::Error::PermissionDenied(
            "need superuser access".to_string(),
        ));
    }

    info!("Setting up namespaces");
    match unsafe { syscalls::syscall!(syscalls::Sysno::unshare, 0x00020000) } {
        Err(error) => return Err(updates::Error::FailedSetupNamespace(error)),
        Ok(_) => {}
    };

    Ok(())
}
