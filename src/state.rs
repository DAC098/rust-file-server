use crate::{db::ArcDBState, template::ArcTemplateState, snowflakes::IdSnowflakes, storage::ArcStorageState, http::Request};

#[derive(Clone)]
pub struct AppState {
    pub db: ArcDBState,
    pub storage: ArcStorageState,
    pub template: ArcTemplateState<'static>,
    pub snowflakes: IdSnowflakes,
    pub offload: tokio::runtime::Handle,
}

impl AppState {
    pub fn from(req: &mut Request) -> AppState {
        req.extensions_mut().remove().unwrap()
    }
}