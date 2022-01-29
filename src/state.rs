use crate::{db::ArcDBState, template::ArcTemplateState, snowflakes::IdSnowflakes, storage::ArcStorageState};

#[derive(Clone)]
pub struct AppState<'a> {
    pub db: ArcDBState,
    pub storage: ArcStorageState,
    pub template: ArcTemplateState<'a>,
    pub snowflakes: IdSnowflakes
}