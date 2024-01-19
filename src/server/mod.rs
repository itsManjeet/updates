use std::error::Error as OtherError;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Mutex;

use ostree::gio::Cancellable;
use tracing::info;
use zbus::{dbus_interface, DBusError};

use crate::engine::Engine;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum Status {
    Idle = 0,
    Checking = 1,
    Deploying = 2,
}

#[derive(Debug)]
pub struct Server {
    engine: Mutex<Engine>,
    status: Status,
}

impl Server {
    pub fn new() -> Result<Server, Error> {
        Ok(Server {
            engine: Engine::new(&PathBuf::from("/"))?.into(),
            status: Status::Idle,
        })
    }
}

#[dbus_interface(name = "dev.rlxos.updates")]
impl Server {
    #[dbus_interface(property)]
    async fn status(&self) -> u8 {
        self.status as u8
    }

    async fn check(&mut self) -> Result<(bool, String), Error> {
        if self.status != Status::Idle {
            return Err(Error::EngineIsBusy);
        }

        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            self.status = Status::Checking;
            let result = engine.check(None, Cancellable::NONE);
            engine.unlock();
            self.status = Status::Idle;
            let (changed, changelog) = result?;

            Ok((changed, changelog))
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn apply(&mut self) -> Result<bool, Error> {
        if self.status != Status::Idle {
            return Err(Error::EngineIsBusy);
        }

        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            self.status = Status::Deploying;
            let result = engine.apply(None, Cancellable::NONE);
            self.status = Status::Idle;

            engine.unlock();

            let changed = result?;

            Ok(changed)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn state(&mut self) -> Result<Vec<((String, String), Vec<(String, String)>)>, Error> {
        if let Ok(engine) = self.engine.lock() {
            let mut result: Vec<((String, String), Vec<(String, String)>)> = Vec::new();
            for state in engine.states()? {
                let mut extensions_list: Vec<(String, String)> = Vec::new();
                for extension in state.extensions {
                    extensions_list.push((extension.refspec.clone(), extension.revision.clone()));
                }

                result.push(((state.core.refspec.clone(), state.core.revision.clone()), extensions_list));
            }
            Ok(result)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn switch(&mut self, channel: &str) -> Result<bool, Error> {
        if self.status != Status::Idle {
            return Err(Error::EngineIsBusy);
        }

        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            self.status = Status::Deploying;
            let result = engine.switch(channel, None, Cancellable::NONE);
            self.status = Status::Idle;

            engine.unlock();

            let changed = result?;

            Ok(changed)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn reset(&mut self, channel: &str) -> Result<bool, Error> {
        if self.status != Status::Idle {
            return Err(Error::EngineIsBusy);
        }

        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            self.status = Status::Deploying;
            let result = engine.reset(channel, None, Cancellable::NONE);
            self.status = Status::Idle;

            engine.unlock();

            let changed = result?;

            Ok(changed)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn add_extension(&mut self, extensions: Vec<String>) -> Result<bool, Error> {
        if self.status != Status::Idle {
            return Err(Error::EngineIsBusy);
        }

        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;
            info!("Adding extensions: {:?}", extensions);

            self.status = Status::Deploying;
            let result = engine.add_extension(extensions, None, Cancellable::NONE);
            self.status = Status::Idle;

            engine.unlock();

            let changed = result?;

            Ok(changed)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn list(&mut self) -> Result<Vec<String>, Error> {
        if let Ok(engine) = self.engine.lock() {
            let list = engine.list(None, Cancellable::NONE)?;
            Ok(list)
        } else {
            Err(Error::EngineIsBusy)
        }
    }
}

#[derive(Debug, DBusError)]
pub enum Error {
    Engine(String),
    EngineIsBusy,
}

impl From<crate::Error> for Error {
    fn from(value: crate::Error) -> Self {
        Error::Engine(get_error_str(value))
    }
}

fn get_error_str(error: crate::Error) -> String {
    let sources = sources(&error);
    let error = sources.join(": ");
    format!("ERROR: {error}")
}

fn sources(error: &crate::Error) -> Vec<String> {
    let mut sources = vec![error.to_string()];
    let mut source = error.source();
    while let Some(error) = source.take() {
        sources.push(error.to_string());
        source = error.source();
    }
    sources
}
