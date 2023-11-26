use std::{collections::HashMap, path::PathBuf, process::Command};

use indicatif::ProgressBar;

use crate::{
    downloader::{self, download},
    element::Element,
};

pub async fn download_sources(
    sources: &Vec<String>,
    path: &PathBuf,
    srcdir: &PathBuf,
    progress: Option<&ProgressBar>,
) -> Result<(), downloader::Error> {
    for src in sources {
        let (url, file) = match src.contains("::") {
            true => {
                let idx = src.find("::").unwrap();
                let url: String = src.chars().take(idx).collect();
                let file: String = src.chars().skip(idx + 2).collect();

                (url, file)
            }
            false => (
                src.clone(),
                String::from(PathBuf::from(src).file_name().unwrap().to_str().unwrap()),
            ),
        };
        let filepath = path.join(file);
        download(&url, &filepath, progress).await?;
        if let Some(ext) = filepath.extension() {
            if vec![".xz", ".tar", ".zip", ".gz"].contains(&ext.to_str().unwrap()) {
                Command::new("bsdtar")
                    .arg("-xf")
                    .arg(&filepath)
                    .arg("-C")
                    .arg(&srcdir)
                    .spawn()?;
            }
        }
    }
    Ok(())
}

pub async fn build(
    element: &Element,
    path: &PathBuf,
    environ: &HashMap<String, String>,
    variables: &HashMap<String, String>,
    progress: Option<&ProgressBar>,
) -> Result<(), crate::engine::Error> {
    let work_dir = path.join("work");

    if let Some(sources) = &element.sources {
        download_sources(&sources, &path, &work_dir, progress).await?;
    }
    let config = match &element.config {
        Some(config) => config.clone(),
        None => HashMap::new(),
    };

    let build_dir = work_dir.join(match &config.contains_key("build-dir") {
        true => config["build-dir"].clone(),
        false => String::from(""),
    });

    let script = match &config.contains_key("script") {
        true => config["script"].clone(),
        false => String::from(""),
    };

    Command::new("sh")
        .arg("-c")
        .arg(&script)
        .current_dir(build_dir)
        .env_clear()
        .envs(environ)
        .spawn()?;

    Ok(())
}
