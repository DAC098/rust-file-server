mod get;
mod post;
mod delete;
mod put;

pub use get::handle_get;
pub use post::handle_post;
pub use delete::handle_delete;
pub use put::handle_put;