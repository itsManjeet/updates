use crate::{database::Database, meta::MetaInfo, repository::Repository};

use console::{style, Emoji};
use indicatif::ProgressBar;
use std::{collections::HashSet, fs, io, path::PathBuf, process::Command, time::Duration};
use thiserror::Error;

static CLOUD: Emoji<'_, '_> = Emoji("☁️   ", "");

pub enum ListMode {
    Installed,
    Remote,
    Outdated,
    Matched(String),
}

pub struct Engine {
    server: String,
    root: PathBuf,
    repo: Repository,
    db: Database,

    pub progress: Option<ProgressBar>,
}

const DB_PATH: &str = "usr/share/swupd/repo";
const CACHE_PATH: &str = "var/cache/swupd/packages";

impl Engine {
    pub fn new(root: impl Into<PathBuf>, server: &str) -> Engine {
        let root = root.into();
        let db_path = root.join(DB_PATH);
        Engine {
            server: String::from(server),
            root: root,
            repo: Repository::new(),
            db: Database::new(db_path),
            progress: None,
        }
    }

    pub fn set_progress(&mut self, progress: ProgressBar) {
        self.progress = Some(progress);
    }

    pub async fn load(&mut self) -> Result<(), Error> {
        self.db.refresh().await?;
        Ok(())
    }

    pub async fn sync(&mut self) -> Result<(), Error> {
        self.repo
            .refresh(&format!("{}/origin", self.server))
            .await?;
        Ok(())
    }

    pub async fn remove(&mut self, ids: &Vec<String>) -> Result<(), Error> {
        for id in ids {
            if let Some(package) = self.db.get(id) {
                if let Some(mut files) = self.db.files(&package.id)? {
                    files.reverse();
                    if let Some(progress) = &self.progress {
                        progress.reset();
                        progress.set_length(100);
                        progress.set_message(format!("REMOVING {}", id));
                    }
                    let mut count = 0;
                    for file in &files {
                        if let Some(progress) = &self.progress {
                            progress
                                .set_position(((count as f32 / files.len() as f32) * 100.0) as u64);
                        }
                        if !self.root.join(&file).is_dir() {
                            fs::remove_file(self.root.join(&file)).ok();
                        }
                        count += 1;
                    }
                    self.db.remove(&package.id.clone()).await?;
                    if let Some(progress) = &self.progress {
                        progress.finish_with_message(format!("SUCCESS {}", id));
                        println!();
                    }
                } else {
                    return Err(Error::MissingFilesDatabase(id.clone()));
                }
            }
        }

        Ok(())
    }

    pub async fn list(&self, mode: ListMode) -> Result<Vec<MetaInfo>, Error> {
        let packages: Vec<MetaInfo> = match mode {
            ListMode::Installed => self.db.iter().map(|v| v.clone()).collect(),
            ListMode::Remote => self.repo.iter().map(|v| v.clone()).collect(),
            ListMode::Outdated => {
                let mut packages = Vec::<MetaInfo>::new();
                for p in self.db.iter() {
                    if let Some(i) = self.repo.get(&p.id) {
                        if i == p {
                            packages.push(i.clone());
                        }
                    }
                }
                packages
            }
            ListMode::Matched(m) => {
                let mut packages = Vec::<MetaInfo>::new();
                for p in self.repo.iter() {
                    if p.id.contains(&m) || p.about.contains(&m) {
                        packages.push(p.clone());
                    }
                }
                packages
            }
        };

        Ok(packages)
    }

    pub async fn install(&mut self, packages: &Vec<MetaInfo>) -> Result<(), Error> {
        let mut files_to_clean: Vec<String> = Vec::new();

        fs::create_dir_all(self.root.join(CACHE_PATH))?;

        for (idx, package) in packages.iter().enumerate() {
            let package_url = format!("{}/cache/{}", self.server, package.cache);
            let package_path = self.root.join(CACHE_PATH).join(&package.cache);

            if let Some(progress) = &self.progress {
                progress.reset();
                progress.set_prefix(format!(
                    "[{}/{}] {} {:width$}",
                    idx + 1,
                    packages.len(),
                    &package.id,
                    ' ',
                    width = 100 - &package.id.len(),
                ));
                progress.set_message("downloading...");
            }

            if !package_path.exists() {
                self.repo
                    .download(package_url.as_str(), &package_path, self.progress.as_ref())
                    .await?;
            }

            if let Some(progress) = &self.progress {
                progress.set_prefix(format!(
                    "[{}/{}] {} {:<4}",
                    idx + 1,
                    packages.len(),
                    &package.id,
                    ' '
                ));
                progress.set_message("loading content...");
            }
            let output = Command::new("tar").arg("-tf").arg(&package_path).output()?;
            if !output.status.success() {
                return Err(Error::InvalidPackage(
                    package_path.display().to_string(),
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
            let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .to_string()
                .split("\n")
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            if let Some(progress) = &self.progress {
                progress.set_message("collecting deprecated...");
            }

            if let Some(old_files) = self.db.files(&package.id)? {
                for file in old_files {
                    if !files.contains(&file) {
                        files_to_clean.push(file.clone());
                    }
                }
            }

            if let Some(progress) = &self.progress {
                progress.set_message("extracting package...");
            }
            Command::new("tar")
                .arg("-xhPf")
                .arg(&package_path)
                .arg("-C")
                .arg(&self.root)
                .spawn()?;

            if !package.integration.is_empty() {
                if let Some(progress) = &self.progress {
                    progress.set_message("integration...");
                }
                Command::new("sh")
                    .arg("-c")
                    .arg(&package.integration)
                    .spawn()?;
            }

            if let Some(progress) = &self.progress {
                progress.set_position(100);
                progress.set_message("registering...");
            }

            self.db.add(&package, &files).await?;
            std::thread::sleep(Duration::from_millis(50));

            if let Some(progress) = &self.progress {
                progress.set_message("finished");
                println!();
            }
        }
        files_to_clean.reverse();
        for file in files_to_clean {
            if PathBuf::from(&file).is_dir() {
                continue;
            }
            fs::remove_file(file).ok();
        }

        Ok(())
    }

    fn resolve_(
        &self,
        id: &String,
        visited: &mut HashSet<String>,
        result: &mut Vec<MetaInfo>,
    ) -> Result<(), Error> {
        if !visited.contains(id) {
            visited.insert(id.clone());

            if let Some(info) = self.repo.get(id) {
                if let Some(installed_info) = self.db.get(id) {
                    if info == installed_info {
                        return Ok(());
                    }
                }

                for dep in &info.depends {
                    match self.resolve_(dep, visited, result) {
                        Ok(_) => {}
                        Err(error) => return Err(Error::DependencyFailed(error.to_string())),
                    }
                }
                result.push(info.clone());
            } else {
                return Err(Error::MissingComponent(id.clone()));
            }
        }
        Ok(())
    }
    pub async fn resolve(&self, ids: &Vec<String>) -> Result<Vec<MetaInfo>, Error> {
        let mut packages: Vec<MetaInfo> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();

        for id in ids {
            self.resolve_(id, &mut visited, &mut packages)?;
        }
        Ok(packages)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Internal IO Error")]
    IO(#[from] io::Error),

    #[error("Database Error")]
    DB(#[from] crate::database::Error),

    #[error("Remote Repository Error")]
    Repository(#[from] crate::repository::Error),

    #[error("Missing component {0}")]
    MissingComponent(String),

    #[error("Dependency failed {0}")]
    DependencyFailed(String),

    #[error("Invalid package {0} {1}")]
    InvalidPackage(String, String),

    #[error("Missing files database {0}")]
    MissingFilesDatabase(String),

    #[error("Installation failed {0}")]
    InstallationFailed(String),
}
