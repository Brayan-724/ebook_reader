use env_logger::Builder;
use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::config::EbookConfig;
use crate::error::{EbookError, EbookResult};

pub struct Logger {
    level: Level,
    stdout_logger: env_logger::Logger,
    file_logger: Option<env_logger::Logger>,
}

impl Logger {
    fn new_builder() -> Builder {
        let mut builder = Builder::from_default_env();
        builder.filter(Some("rustls::client::hs"), LevelFilter::Off);
        builder
    }

    pub fn new(config: &EbookConfig) -> Self {
        let level = Level::Trace;
        let a = Self::new_builder().build();
        let b = if let Some(log_file) = &config.log_file {
            Some(
                Self::new_builder()
                    .filter(None, LevelFilter::Trace)
                    .target(env_logger::Target::Pipe(Box::from(
                        std::fs::File::create(log_file).unwrap(),
                    )))
                    .build(),
            )
        } else {
            None
        };

        Self {
            level,
            stdout_logger: a,
            file_logger: b,
        }
    }

    pub fn init(config: &EbookConfig) -> EbookResult<()> {
        let logger = Self::new(config);
        log::set_max_level(LevelFilter::Trace);
        if let Err(..) = log::set_boxed_logger(Box::from(logger)) {
            Err(EbookError::LoggerAlreadyInitialized)
        } else {
            Ok(())
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            self.stdout_logger.log(record);
            if let Some(file_logger) = &self.file_logger {
                file_logger.log(record);
            }
        }
    }

    fn flush(&self) {
        self.stdout_logger.flush();
        if let Some(file_logger) = &self.file_logger {
            file_logger.flush();
        }
    }
}
