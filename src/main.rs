use std::error::Error;
use std::future::pending;
use std::path::PathBuf;
use std::string::ToString;

use zbus::ConnectionBuilder;

use updatectl::engine::Engine;
use updatectl::server::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_namespaces()?;

    let engine = Engine::new(&PathBuf::from("/"))?;
    let server = Server { engine: engine.into() };

    let _conn = ConnectionBuilder::system()?.name("dev.rlxos.updates")?.serve_at("/dev/rlxos/updates", server)?.build().await?;

    println!("listening...");
    pending::<()>().await;
    Ok(())
}

pub fn setup_namespaces() -> Result<(), updatectl::Error> {
    if nix::unistd::getegid().as_raw() != 0 {
        return Err(updatectl::Error::PermissionDenied("need superuser access".to_string()));
    }

    match unsafe { syscalls::syscall!(syscalls::Sysno::unshare, 0x00020000) } {
        Err(error) => return Err(updatectl::Error::FailedSetupNamespace(error)),
        Ok(_) => {}
    };

    Ok(())
}
