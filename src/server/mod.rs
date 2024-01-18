use std::fmt::Debug;
use std::sync::Mutex;

use zbus::{dbus_interface, DBusError};
use ostree::gio::Cancellable;
use crate::engine::Engine;

#[derive(Debug)]
pub struct Server {
    pub engine: Mutex<Engine>,
}

#[dbus_interface(name = "dev.rlxos.updates")]
impl Server {
    async fn check(&mut self) -> Result<(bool, String), Error> {
        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            let result = engine.check(None, Cancellable::NONE);
            engine.unlock();

            let (changed, changelog) = result?;

            Ok((changed, changelog))
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn apply(&mut self) -> Result<bool, Error> {
        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            let result = engine.apply(None, Cancellable::NONE);

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
                if let Some(extensions) = &state.extensions {
                    for extension in extensions {
                        extensions_list.push((extension.refspec.clone(), extension.revision.clone()));
                    }
                }

                result.push(((state.core.refspec.clone(), state.core.revision.clone()), extensions_list));
            }
            Ok(result)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn switch(&mut self, channel: &str) -> Result<bool, Error> {
        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            let result = engine.switch(channel, None, Cancellable::NONE);

            engine.unlock();

            let changed = result?;

            Ok(changed)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn reset(&mut self, channel: &str) -> Result<bool, Error> {
        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            let result = engine.reset(channel, None, Cancellable::NONE);

            engine.unlock();

            let changed = result?;

            Ok(changed)
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn add_extension(&mut self, extensions: Vec<String>) -> Result<bool, Error> {
        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            let result = engine.add_extension(extensions, None, Cancellable::NONE);

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
        Error::Engine(value.to_string())
    }
}
