use argh::FromArgs;
use std::{env, time::Duration};

const DEFAULT_TARGET_FPS: u16 = 60;
const DEFAULT_MAX_EVENTS_PER_BATCH: usize = 32;
const TARGET_FPS_ENV: &str = "ALERT_TUI_TARGET_FPS";

#[derive(FromArgs, Debug, Clone)]
#[argh(description = "Alert TUI")]
pub struct Args {
    #[argh(option, short = 'f', description = "target fps cap")]
    pub target_fps: Option<u16>,
}

#[derive(Clone, Copy, Debug)]
pub struct UiConfig {
    pub target_fps: u16,
    pub max_events_per_batch: usize,
}

impl UiConfig {
    pub fn from_args(args: &Args) -> Self {
        let target_fps = args
            .target_fps
            .or_else(|| {
                env::var(TARGET_FPS_ENV)
                    .ok()
                    .and_then(|value| value.parse().ok())
            })
            .filter(|fps| *fps > 0)
            .unwrap_or(DEFAULT_TARGET_FPS);

        Self {
            target_fps,
            max_events_per_batch: DEFAULT_MAX_EVENTS_PER_BATCH,
        }
    }

    pub fn frame_interval(self) -> Duration {
        Duration::from_secs_f64(1.0 / f64::from(self.target_fps))
    }
}
