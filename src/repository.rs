use futures_util::StreamExt;
use indicatif::{HumanBytes, ProgressBar};
use std::cmp::min;
use std::fs::rename;
use std::io;
use std::io::Write;
use std::{fs::File, path::PathBuf};
use thiserror::Error;

use crate::meta::MetaInfo;

#[derive(Debug)]
pub struct Repository {
    packages: Vec<MetaInfo>,
}

impl Repository {
    pub fn new() -> Repository {
        Repository {
            packages: Vec::new(),
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MetaInfo> {
        self.packages.iter()
    }

    pub async fn refresh(&mut self, url: &str) -> Result<(), Error> {
        self.packages.clear();
        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            let mesg = match response.status().canonical_reason() {
                Some(mesg) => mesg,
                None => "unknown",
            };

            return Err(Error::InvalidUrl(String::from(url), String::from(mesg)));
        }

        self.packages = response.json().await?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&MetaInfo> {
        for p in self.packages.iter() {
            if p.id == id || format!("{}.yml", p.id) == id || format!("components/{}", id) == p.id {
                return Some(p);
            }
        }
        None
    }

    pub async fn download(
        &self,
        url: &str,
        filepath: &PathBuf,
        progress: Option<&ProgressBar>,
    ) -> Result<(), Error> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;

        if response.status().is_success() {
            let mut tmpfile_path = filepath.display().to_string();
            tmpfile_path.push_str(".tmp");

            let tmpfile_path = PathBuf::from(tmpfile_path);
            let mut outfile = File::create(&tmpfile_path)?;
            let total_size = response.content_length().ok_or(Error::InvalidUrl(
                String::from(url),
                String::from("failed to get content length"),
            ))?;

            let mut stream = response.bytes_stream();
            let mut downloaded: u64 = 0;

            while let Some(item) = stream.next().await {
                let chunk = item?;
                outfile.write_all(&chunk)?;
                let new = min(downloaded + (chunk.len() as u64), total_size);
                downloaded = new;
                if let Some(progress) = progress {
                    progress.inc(1);
                    progress.set_message(format!(
                        "[{}/{}]",
                        HumanBytes(downloaded),
                        HumanBytes(total_size)
                    ));
                }
            }

            rename(tmpfile_path, filepath)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Request failed")]
    Request(#[from] reqwest::Error),

    #[error("Invalid url {0} {1}")]
    InvalidUrl(String, String),

    #[error("System IO Error")]
    IO(#[from] io::Error),
}
