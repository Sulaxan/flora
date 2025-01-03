use std::{fs, path::PathBuf, sync::atomic::Ordering};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{CONTENT, CONTENT_URL, HEIGHT, NAME, POS_X, POS_Y, WIDTH};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// A custom name for the widget. This is used to easily identify the widget for the end user.
    name: Option<String>,
    /// The position of the widget
    #[serde(rename = "pos")]
    position: Option<(i32, i32)>,
    /// The dimensions of the widget
    #[serde(rename = "dim")]
    dimension: Option<(i32, i32)>,
    /// The HTML or URL content of the widget
    content: String,
    /// Whether to treat the content as a URL
    content_url: Option<bool>,
}

pub fn parse(config: &str) -> Result<Config> {
    Ok(serde_lexpr::from_str(config)?)
}

pub fn read(path: &PathBuf) -> Result<Config> {
    serde_lexpr::from_slice(&fs::read(path).with_context(|| "could not read path")?)
        .with_context(|| "could not parse config")
}

pub fn load_config(config: Config) {
    if let Some(name) = config.name {
        let mut n = NAME.lock().unwrap();
        *n = name;
    }
    if let Some(position) = config.position {
        POS_X.store(position.0, Ordering::SeqCst);
        POS_Y.store(position.1, Ordering::SeqCst);
    }
    if let Some(dim) = config.dimension {
        WIDTH.store(dim.0, Ordering::SeqCst);
        HEIGHT.store(dim.1, Ordering::SeqCst);
    }

    {
        let mut c = CONTENT.lock().unwrap();
        *c = config.content;
    }

    if let Some(_) = config.content_url {
        CONTENT_URL.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::parse;

    #[test]
    fn test_print() {
        let config = Config {
            name: Some("flora".to_string()),
            position: Some((100, 0)),
            dimension: Some((200, 0)),
            content: "".to_string(),
            content_url: Some(false),
        };

        println!("Config: {}", serde_lexpr::to_string(&config).unwrap());
    }

    #[test]
    fn test_parse() {
        let config = parse("((pos #(0 0))\n(dim #(200 0)) (content . \"\"))").unwrap();

        assert_eq!(
            config,
            Config {
                name: None,
                position: Some((0, 0)),
                dimension: Some((200, 0)),
                content: "".to_string(),
                content_url: None,
            }
        )
    }
}
