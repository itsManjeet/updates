use std::sync::Mutex;

use ostree::gio::Cancellable;
use zbus::{dbus_interface, DBusError, Message, MessageHeader};
use zbus::names::ErrorName;

use crate::engine::Engine;
use crate::Error;

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

            result
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn apply(&mut self) -> Result<bool, Error> {
        if let Ok(engine) = self.engine.lock() {
            engine.lock()?;

            engine.unlock();

            result
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn state(&mut self) -> Result<((String, String), Vec<(String, String)>), Error> {
        if let Ok(engine) = self.engine.lock() {
            let state = engine.state()?;
            let mut extensions_list: Vec<(String, String)> = Vec::new();
            if let Some(extensions) = &state.extensions {
                for extension in extensions {
                    extensions_list.push((extension.refspec.clone(), extension.revision.clone()));
                }
            }

            Ok(((state.core.refspec.clone(), state.core.revision.clone()), extensions_list))
        } else {
            Err(Error::EngineIsBusy)
        }
    }

    async fn list(&mut self) -> Result<Vec<String>, Error> {
        if let Ok(engine) = self.engine.lock() {
            engine.list(None, Cancellable::NONE)
        } else {
            Err(Error::EngineIsBusy)
        }
    }
}


impl DBusError for Error {
    fn create_reply(&self, msg: &MessageHeader<'_>) -> zbus::Result<Message> {
        todo!()
    }

    fn name(&self) -> ErrorName<'_> {
        todo!()
    }

    fn description(&self) -> Option<&str> {
        todo!()
    }
}