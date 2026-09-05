use std::sync::Mutex;

use crate::model::PersistedState;

pub struct AppState {
    inner: Mutex<PersistedState>,
}

impl AppState {
    pub fn new(state: PersistedState) -> Self {
        Self {
            inner: Mutex::new(state),
        }
    }

    pub fn snapshot(&self) -> Result<PersistedState, String> {
        self.inner
            .lock()
            .map(|state| state.clone())
            .map_err(|_| "No se pudo acceder al estado de la aplicación".to_owned())
    }

    pub fn update<F>(&self, update: F) -> Result<PersistedState, String>
    where
        F: FnOnce(&mut PersistedState) -> Result<(), String>,
    {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "No se pudo acceder al estado de la aplicación".to_owned())?;
        update(&mut state)?;
        Ok(state.clone())
    }
}
