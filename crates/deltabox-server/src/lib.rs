mod app;
mod error;
mod handlers;

pub use app::build_router;
pub use error::ApiError;

use deltabox_core::Vault;

#[derive(Clone)]
pub struct AppState {
    pub vault: Vault,
}
