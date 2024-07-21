use std::process::id;

pub enum LaunchLocation {
    Terminal,
    UserClick,
}

#[cfg(target_os = "windows")]
pub mod launch_check {}

#[cfg(not(target_os = "windows"))]
pub mod launch_check {}
