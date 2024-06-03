use std::env::{self, VarError};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::{EbookError, EbookResult};

const STREAM_KEY_NAME: &str = "TWITCH_STREAM_KEY";
const PREVIEW_NAME: &str = "PREVIEW";
const LOG_FILE_NAME: &str = "LOG_FILE";

#[derive(Debug)]
pub struct EbookConfig {
    pub stream_key: String,
    pub preview: bool,
    pub log_file: Option<PathBuf>,
}

impl fmt::Display for EbookConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const RST_: &str = "\x1b[0m";
        const RED_: &str = "\x1b[1;31m";
        const GREE: &str = "\x1b[1;32m";
        const YELL: &str = "\x1b[1;33m";

        write!(f, "{GREE}Configuration:{RST_}\n")?;
        write!(
            f,
            "  {YELL}Stream key : {RED_}{}{RST_}\n",
            self.stream_key.chars().map(|_| '*').collect::<String>()
        )?;
        write!(f, "  {YELL}Preview    : {GREE}{}{RST_}\n", self.preview)?;
        if let Some(log_file) = &self.log_file {
            write!(
                f,
                "  {YELL}Log File   : {GREE}{}{RST_}\n",
                log_file.display()
            )?;
        } else {
            write!(f, "  {YELL}Log File   : {RED_}No{RST_}\n")?;
        }

        Ok(())
    }
}

impl EbookConfig {
    pub fn from_envs() -> EbookResult<Self> {
        Ok(Self {
            stream_key: load_env(STREAM_KEY_NAME)?.ok_or(EbookError::NoTwitchStreamKey)?,
            preview: load_bool(PREVIEW_NAME)?.unwrap_or(false),
            log_file: load_env(LOG_FILE_NAME)?.map(|p| p.into()),
        })
    }
}

fn load_bool(key: &'static str) -> EbookResult<Option<bool>> {
    let Some(v) = load_env(key)? else {
        return Ok(None);
    };

    Ok(Some(&v != "0" && &v != "false"))
}

fn load_env(key: &'static str) -> EbookResult<Option<String>> {
    match env::var(key) {
        Ok(v) => Ok(Some(v)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(EbookError::InvalidEnvEncoding(key)),
    }
}
