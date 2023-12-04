use std::io;
use std::path::PathBuf;
use thiserror::Error;

use crate::downloader::{self, download};
use crate::meta::MetaInfo;
use crate::progress::Progress;

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
        progres: Option<Progress>,
    ) -> Result<(), Error> {
        download(url, filepath, progres)
            .await
            .map_err(Error::DownloadFailed)
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

    #[error("Download failed")]
    DownloadFailed(#[from] downloader::Error),
}
