use std::{fs, path::PathBuf, sync::atomic::Ordering};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{CONTENT, CONTENT_URL, HEIGHT, POS_X, POS_Y, WIDTH};

pub type Config = Vec<ConfigOption>;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigOption {
    /// The position of the widget
    Position(i32, i32),
    /// The dimensions of the widget
    #[serde(rename = "dim")]
    Dimension(i32, i32),
    /// The HTML or URL content of the widget
    Content(String),
    /// Whether to treat the content as a URL
    ContentUrl,
}

pub fn parse(config: &str) -> Result<Config> {
    Ok(serde_lexpr::from_str(config)?)
}

pub fn read(path: &PathBuf) -> Result<Config> {
    serde_lexpr::from_slice(&fs::read(path).with_context(|| "could not read path")?)
        .with_context(|| "could not parse config")
}

pub fn load_config(config: Config) {
    for option in config {
        match option {
            ConfigOption::Position(x, y) => {
                POS_X.store(x, Ordering::SeqCst);
                POS_Y.store(y, Ordering::SeqCst);
            }
            ConfigOption::Dimension(w, h) => {
                WIDTH.store(w, Ordering::SeqCst);
                HEIGHT.store(h, Ordering::SeqCst);
            }
            ConfigOption::Content(content) => {
                let mut c = CONTENT.lock().unwrap();
                *c = content;
            }
            ConfigOption::ContentUrl => {
                CONTENT_URL.store(true, Ordering::SeqCst);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::ConfigOption;

    use super::parse;

    #[test]
    fn test_parse() {
        let options = parse("((position 0 0)\n(dim 200 0))").unwrap();

        assert_eq!(
            options,
            vec![
                ConfigOption::Position(0, 0),
                ConfigOption::Dimension(200, 0)
            ]
        )
    }
}
