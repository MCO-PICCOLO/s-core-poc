/// Stub implementation of the Eclipse S-CORE `stdout_logger` crate.
///
/// Provides a `StdoutLoggerBuilder` that initialises `env_logger` as
/// the global logger, mirroring the builder API used in mini-adas binaries.
use log::LevelFilter;

pub struct StdoutLoggerBuilder {
    level: LevelFilter,
}

impl StdoutLoggerBuilder {
    pub fn new() -> Self {
        Self {
            level: LevelFilter::Info,
        }
    }

    pub fn context(self, _ctx: &str) -> Self {
        self
    }

    pub fn show_module(self, _v: bool) -> Self {
        self
    }

    pub fn show_file(self, _v: bool) -> Self {
        self
    }

    pub fn show_line(self, _v: bool) -> Self {
        self
    }

    pub fn log_level(mut self, level: LevelFilter) -> Self {
        self.level = level;
        self
    }

    /// Install this logger as the process-global logger.
    ///
    /// Silently ignores `SetLoggerError` (already set) so safe to call
    /// multiple times (e.g. in tests).
    pub fn set_as_default_logger(self) {
        let _ = env_logger::builder()
            .filter_level(self.level)
            .try_init();
    }
}

impl Default for StdoutLoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
