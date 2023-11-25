use crate::meta::MetaInfo;
use chrono::prelude::*;
use std::io::Read;
use std::{
    fs::{self, File},
    io,
    path::PathBuf,
};
use thiserror::Error;

#[derive(Debug)]
pub struct Database {
    path: PathBuf,
    packages: Vec<MetaInfo>,
}

impl Database {
    pub fn new(path: impl Into<PathBuf>) -> Database {
        Database {
            path: path.into(),
            packages: Vec::new(),
        }
    }

    pub fn format(id: &String) -> String {
        id.replace("/", "-")
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MetaInfo> {
        self.packages.iter()
    }

    pub async fn refresh(&mut self) -> Result<(), Error> {
        self.packages.clear();
        if self.path.exists() {
            let iter = fs::read_dir(&self.path)?;
            for dir in iter {
                let dir = dir?;
                if dir.path().is_dir() {
                    let info_file = dir.path().join("info");
                    if info_file.exists() {
                        let reader = File::open(&info_file)?;
                        let package: MetaInfo = serde_yaml::from_reader(reader)?;
                        self.packages.push(package);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&MetaInfo> {
        for p in self.packages.iter() {
            if format!("{}.yml", p.id) == id || p.id == format!("components/{}", id) || p.id == id {
                return Some(&p);
            }
        }
        None
    }

    pub fn files(&self, id: &str) -> Result<Option<Vec<String>>, Error> {
        if let Some(info) = self.get(id) {
            let filepath = self.path.join(Database::format(&info.id)).join("files");
            let mut content = String::new();
            File::open(filepath)?.read_to_string(&mut content)?;
            return Ok(Some(content.split("\n").map(|s| s.to_string()).collect()));
        }
        Ok(None)
    }

    pub async fn remove(&mut self, id: &String) -> Result<(), Error> {
        let path = self.path.join(Database::format(id));
        println!("removing:{}", path.display());
        fs::remove_dir_all(self.path.join(Database::format(id)))?;
        Ok(())
    }

    pub async fn add(&mut self, metainfo: &MetaInfo, files: &Vec<String>) -> Result<(), Error> {
        let content = serde_yaml::to_string(metainfo)?;
        let package_path = self.path.join(Database::format(&metainfo.id));
        fs::create_dir_all(&package_path)?;

        fs::write(package_path.join("info"), content)?;
        fs::write(package_path.join("files"), files.join("\n"))?;
        fs::write(package_path.join("at"), Utc::now().to_string())?;

        self.refresh().await?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("io")]
    IO(#[from] io::Error),

    #[error("yaml")]
    Yaml(#[from] serde_yaml::Error),
}
