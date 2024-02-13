use thiserror::Error;

pub mod cmd;
pub mod engine;
pub mod progress;

#[derive(Debug, Error)]
pub enum Error {
    #[error("glib")]
    GLib(#[from] ostree::glib::Error),

    #[error("no boot deployment")]
    NoBootDeployment,

    #[error("no previous deployment")]
    NoPreviousDeployment,

    #[error("no origin known for deployment {0}.{1}")]
    NoOriginForDeployment(String, i32),

    #[error("no revision for refspec {0}")]
    NoRevisionForRefSpec(String),

    #[error("no base checksum")]
    NoBaseCheckSum,

    #[error("no extension checksum {0}")]
    NoExtCheckSum(String),

    #[error("failed to prepare transaction")]
    FailedPrepareTransaction,

    #[error("permission error {0}")]
    PermissionError(String),

    #[error("failed to lock sysroot")]
    FailedTryLock,

    #[error("failed to setup namespace {0}")]
    FailedSetupNamespace(syscalls::Errno),

    #[error("no remote found")]
    NoRemoteFound,

    #[error("permission denied {0}")]
    PermissionDenied(String),

    #[error("engine is busy")]
    EngineIsBusy,

    #[error("no updates available")]
    NoUpdateAvailable,
}
