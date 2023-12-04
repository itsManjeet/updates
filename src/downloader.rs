use crate::progress::Progress;
use futures_util::StreamExt;
use std::cmp::min;
use std::fs::rename;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;

pub async fn download(
    url: &str,
    filepath: &PathBuf,
    progress: Option<&Progress>,
) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if let Some(progress) = progress {
        progress.process(format!("Downloading {}", url));
    }
    if response.status().is_success() {
        let mut tmpfile_path = filepath.display().to_string();
        tmpfile_path.push_str(".tmp");

        let tmpfile_path = PathBuf::from(tmpfile_path);
        let mut outfile = File::create(&tmpfile_path)?;
        let total_size = match response.content_length() {
            Some(size) => size,
            None => 99999,
        };

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(item) = stream.next().await {
            let chunk = item?;
            outfile.write_all(&chunk)?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            if let Some(progress) = progress {
                progress.update(&format!("downloaded {}/{}", downloaded, total_size));
            }
        }

        rename(tmpfile_path, filepath)?;
    }
    Ok(())
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
