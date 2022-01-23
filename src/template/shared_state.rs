use std::sync::Arc;

use handlebars::{Handlebars, RenderError};
use serde::Serialize;

// so as of writing this, the handlebars render
// processes does support sending information into some form of output but not
// in a streaming fashion and I don't think there is any asynchronous behavior
// either. when a template renders it will be all in memory currently and if
// the template render is large enough then it could cause memory issues

pub struct TemplateState<'a> {
    hb: Handlebars<'a>
}

pub type ArcTemplateState<'a> = Arc<TemplateState<'a>>;

impl<'a> TemplateState<'a> {
    pub fn new(hb: Handlebars<'a>) -> ArcTemplateState<'a> {
        Arc::new(TemplateState { hb })
    }

    pub fn render<T>(
        &self, name: &str, data: &T
    ) -> Result<String, RenderError>
    where
        T: Serialize
    {
        self.hb.render(name, data)
    }
}