//! Profile routing and local app copies for ChatGPT/Codex desktop bundles.
//!
//! This is not a filesystem sandbox or a credential vault.
//! macOS is the runtime target. Unix mock-process tests can also run on Linux.

#[cfg(not(unix))]
compile_error!("Harbor currently supports macOS runtime and Unix-only tests.");

pub mod app;
pub mod auxiliary;
pub mod browser;
pub mod clone;
pub mod environment;
pub mod fsutil;
pub mod icon;
pub mod lifecycle;
pub mod model;
pub mod process;
pub mod store;
pub mod update;

pub use app::AppInfo;
pub use model::{Profile, SCHEMA_VERSION};
pub use process::{LaunchResult, RunningProcess};
pub use store::Store;
