use crate::{db::ArcDBState, template::ArcTemplateState, snowflakes::IdSnowflakes, storage::ArcStorageState};

#[derive(Clone)]
pub struct AppState {
    pub db: ArcDBState,
    pub storage: ArcStorageState,
    pub template: ArcTemplateState<'static>,
    pub snowflakes: IdSnowflakes,
    pub offload: tokio::runtime::Handle,
}