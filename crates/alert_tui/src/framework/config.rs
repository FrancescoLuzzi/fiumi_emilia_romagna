use std::{env, time::Duration};

const DEFAULT_TARGET_FPS: u16 = 60;
const DEFAULT_MAX_EVENTS_PER_BATCH: usize = 32;
const DEFAULT_FILTER_DEBOUNCE_MS: u64 = 175;
const TARGET_FPS_ENV: &str = "ALERT_TUI_TARGET_FPS";

#[derive(Clone, Copy, Debug)]
pub struct UiConfig {
    pub target_fps: u16,
    pub max_events_per_batch: usize,
    pub filter_debounce_ms: u64,
    frame_interval: Duration,
    filter_debounce_interval: Duration,
}

#[derive(Clone, Copy, Debug)]
pub struct UiConfigBuilder {
    target_fps: u16,
    max_events_per_batch: usize,
    filter_debounce_ms: u64,
}

impl UiConfig {
    pub fn builder() -> UiConfigBuilder {
        UiConfigBuilder::default()
    }

    pub fn from_target_fps(target_fps: Option<u16>) -> Self {
        let target_fps = target_fps
            .or_else(|| {
                env::var(TARGET_FPS_ENV)
                    .ok()
                    .and_then(|value| value.parse().ok())
            })
            .filter(|fps| *fps > 0)
            .unwrap_or(DEFAULT_TARGET_FPS);

        Self::builder().target_fps(target_fps).build()
    }

    pub fn frame_interval(self) -> Duration {
        self.frame_interval
    }

    pub fn filter_debounce_interval(self) -> Duration {
        self.filter_debounce_interval
    }
}

impl Default for UiConfigBuilder {
    fn default() -> Self {
        Self {
            target_fps: DEFAULT_TARGET_FPS,
            max_events_per_batch: DEFAULT_MAX_EVENTS_PER_BATCH,
            filter_debounce_ms: DEFAULT_FILTER_DEBOUNCE_MS,
        }
    }
}

impl UiConfigBuilder {
    pub fn target_fps(mut self, target_fps: u16) -> Self {
        if target_fps > 0 {
            self.target_fps = target_fps;
        }
        self
    }

    pub fn max_events_per_batch(mut self, max_events_per_batch: usize) -> Self {
        self.max_events_per_batch = max_events_per_batch;
        self
    }

    pub fn filter_debounce_ms(mut self, filter_debounce_ms: u64) -> Self {
        self.filter_debounce_ms = filter_debounce_ms;
        self
    }

    pub fn build(self) -> UiConfig {
        UiConfig {
            target_fps: self.target_fps,
            max_events_per_batch: self.max_events_per_batch,
            filter_debounce_ms: self.filter_debounce_ms,
            frame_interval: Duration::from_secs_f64(1.0 / f64::from(self.target_fps)),
            filter_debounce_interval: Duration::from_millis(self.filter_debounce_ms),
        }
    }
}
