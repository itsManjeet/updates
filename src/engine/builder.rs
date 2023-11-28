use std::{collections::HashMap, fs, path::PathBuf, process::Command};

use indicatif::ProgressBar;

use crate::{
    downloader::{self, download},
    element::Element,
    engine::Error,
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

#[derive(Clone, Copy)]
enum BuildType {
    AutoTools,
    Meson,
    Cmake,
    Makefile,
    Python,
}

impl BuildType {
    fn detect(path: &PathBuf) -> Option<BuildType> {
        let data = HashMap::from([
            ("configure", BuildType::AutoTools),
            ("meson.build", BuildType::Meson),
            ("CMakeLists.txt", BuildType::Cmake),
            ("Makefile", BuildType::Makefile),
            ("setup.py", BuildType::Python),
        ]);

        for (file, build_type) in data.into_iter() {
            if path.join(file).exists() {
                return Some(build_type);
            }
        }

        None
    }

    fn from_str(id: &str) -> Option<BuildType> {
        let data = HashMap::from([
            ("autotools", BuildType::AutoTools),
            ("meson", BuildType::Meson),
            ("cmake", BuildType::Cmake),
            ("make", BuildType::Makefile),
            ("pip", BuildType::Python),
        ]);

        if data.contains_key(id) {
            return Some(data[id]);
        }

        None
    }

    fn script(&self) -> String {
        match self {
            BuildType::AutoTools => String::from(
                "
if [[ %{seperate-build-dir} == true ]] ; then
    CONFIGURE=../configure
    cd %{build-dir}
else
    CONFIGURE=./configure
fi

$CONFIGURE --prefix=%{prefix} \
    --sysconfdir=%{sysconfdir} \
    --libdir=%{libdir}  \
    --libexecdir=%{libdir} \
    --bindir=%{bindir} \
    --sbindir=%{bindir}
    %{conf-extra}

make -j$(nproc) %{compile}
make install DESTDIR=%{install-root} %{install}
            ",
            ),
            BuildType::Cmake => String::from(
                "
cmake -B %{build-dir} -DCMAKE_INSTALL_PREFIX=%{prefix}
cmake --build %{build-dir} %{compile}
DESTDIR=%{install-root} cmake --install %{build-dir} %{install}
            ",
            ),
            BuildType::Makefile => String::from("
make PREFIX=%{prefix} BINDIR=%{bindir} LIBDIR=%{libdir} LIBEXECDIR=%{libdir} SBINDIR=%{bindir} %{compile}
make PREFIX=%{prefix} BINDIR=%{bindir} LIBDIR=%{libdir} LIBEXECDIR=%{libdir} SBINDIR=%{bindir} install DESTDIR=%{install-root} %{install}
"),
            BuildType::Meson => String::from("
meson %{build-dir} --prefix=%{prefix} \
    --sysconfdir=%{sysconfdir} \
    --libdir=%{libdir}  \
    --libexecdir=%{libdir} \
    --bindir=%{bindir} \
    --sbindir=%{bindir}
    %{conf-extra}

ninja -C %{build-dir} %{compile}
DESTDIR=%{install-root} ninja -C %{build-dir} install %{install}"),
            BuildType::Python => String::from(""),
        }
    }
}

pub fn generate_script(config: &HashMap<String, String>, path: &PathBuf) -> Result<String, Error> {
    match config.contains_key("script") {
        true => Ok(config["script"].clone()),
        false => {
            let build_type = match config.contains_key("build-type") {
                true => BuildType::from_str(config["build-type"].as_str()),
                false => BuildType::detect(&path),
            };

            let script = match build_type {
                Some(build_type) => build_type.script(),
                None => return Err(Error::CompilationFailed),
            };

            Ok(script)
        }
    }
}

pub async fn build(
    element: &Element,
    path: &PathBuf,
    variables: &HashMap<String, String>,
    build_hash: &String,
    progress: Option<&ProgressBar>,
) -> Result<PathBuf, Error> {
    let work_dir = path.join("work");
    let build_root = work_dir.join("build-root");
    let install_root = work_dir.join("install-root");

    fs::create_dir_all(&build_root)?;
    fs::create_dir_all(&install_root)?;

    let mut variables = variables.clone();
    variables.insert(
        String::from("install-root"),
        install_root.display().to_string().clone(),
    );
    variables.insert(
        String::from("build-root"),
        install_root.display().to_string().clone(),
    );

    variables.insert(String::from("prefix"), String::from("/usr"));
    variables.insert(String::from("sysconfdir"), String::from("/etc"));
    variables.insert(String::from("libdir"), String::from("%{prefix}/lib"));
    variables.insert(String::from("bindir"), String::from("%{prefix}/bin"));

    let environ: HashMap<String, String> = HashMap::new();
    let element = element.resolve(&environ, &variables)?;

    if let Some(sources) = &element.sources {
        download_sources(&sources, &path, &work_dir, progress).await?;
    }

    let config = match &element.config {
        Some(config) => config.clone(),
        None => HashMap::new(),
    };

    let script = generate_script(&config, &build_root)?;
    variables.insert(String::from("InternalBuildScript"), script);
    let element = element.resolve(&environ, &variables)?;

    let script = element.config.unwrap()["InternalBuildScript"].clone();

    let build_dir = work_dir.join(match &config.contains_key("build-dir") {
        true => config["build-dir"].clone(),
        false => String::from(""),
    });

    let environ = match &element.environ {
        Some(environ) => environ.clone(),
        None => HashMap::new(),
    };

    println!("script: {}", &script);

    if !Command::new("sh")
        .arg("-c")
        .arg(&script)
        .current_dir(&build_dir)
        .env_clear()
        .envs(environ)
        .spawn()?
        .wait()?
        .success()
    {
        return Err(Error::CompilationFailed);
    }

    let package_path = path.join(build_hash);
    if !Command::new("tar")
        .arg("-I")
        .arg("zstd")
        .arg("-caf")
        .arg(&package_path)
        .arg("-C")
        .arg(&build_dir)
        .arg(".")
        .spawn()?
        .wait()?
        .success()
    {
        return Err(Error::PackagingFailed);
    }

    Ok(package_path)
}
