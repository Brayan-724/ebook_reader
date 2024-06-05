use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum EbookError {
    // Config
    InvalidEnvEncoding(&'static str),
    NoTwitchStreamKey,

    // Glib
    Glib(glib::Error),
    GlibBool(glib::BoolError),

    // Logger
    LoggerAlreadyInitialized,
}

impl fmt::Display for EbookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Config
            Self::InvalidEnvEncoding(key) => write!(f, "Cannot get environment variable {key}.\nIt was found but is not encoded correctly"),
            Self::NoTwitchStreamKey => f.write_str("No Twitch stream key in environment variables.\nTry TWITCH_STREAM_KEY={YOUR_STREAM_KEY}"),

            // Glib
            Self::Glib(err) => write!(f, "Glib Error: {err}"),
            Self::GlibBool(err) => write!(f, "GlibBool Error: {err}"),

            // Logger
            Self::LoggerAlreadyInitialized => f.write_str("Logger was already initialized"),
        }
    }
}

impl Error for EbookError {}

pub type EbookResult<T> = Result<T, EbookError>;
