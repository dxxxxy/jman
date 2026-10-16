//! Downloading JDKs through the foojay Disco API (https://api.foojay.io).

use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::detect::{self, Jdk};
use crate::{link, paths};

const API: &str = "https://api.foojay.io/disco/v3.0";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Distribution {
    pub id: &'static str,
    pub name: &'static str,
}

/// The distributions offered for download, most popular first.
pub const DISTRIBUTIONS: &[Distribution] = &[
    Distribution { id: "temurin", name: "Eclipse Temurin" },
    Distribution { id: "zulu", name: "Azul Zulu" },
    Distribution { id: "corretto", name: "Amazon Corretto" },
    Distribution { id: "microsoft", name: "Microsoft Build of OpenJDK" },
    Distribution { id: "liberica", name: "BellSoft Liberica" },
    Distribution { id: "oracle_open_jdk", name: "Oracle OpenJDK" },
    Distribution { id: "graalvm_community", name: "GraalVM Community" },
    Distribution { id: "sap_machine", name: "SapMachine" },
    Distribution { id: "semeru", name: "IBM Semeru" },
    Distribution { id: "dragonwell", name: "Alibaba Dragonwell" },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct MajorVersion {
    pub major_version: u32,
    pub term_of_support: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Package {
    pub id: String,
    pub distribution: String,
    pub major_version: u32,
    pub java_version: String,
    pub archive_type: String,
    pub filename: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Deserialize)]
struct PackageInfo {
    direct_download_uri: String,
    #[serde(default)]
    checksum: String,
    #[serde(default)]
    checksum_type: String,
}

#[derive(Debug, Deserialize)]
struct ApiResult<T> {
    result: Vec<T>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum Progress {
    Downloading { done: u64, total: u64 },
    Verifying,
    Extracting,
}

fn get<T: for<'de> Deserialize<'de>>(url: &str) -> anyhow::Result<Vec<T>> {
    let res: ApiResult<T> = ureq::get(url)
        .header("User-Agent", concat!("jman/", env!("CARGO_PKG_VERSION")))
        .call()
        .with_context(|| format!("requesting {url}"))?
        .body_mut()
        .read_json()?;
    Ok(res.result)
}

fn os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        _ => "linux",
    }
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => other,
    }
}

fn archive_type() -> &'static str {
    if cfg!(windows) { "zip" } else { "tar.gz" }
}

/// Maintained, generally available Java versions, newest first.
pub fn major_versions() -> anyhow::Result<Vec<MajorVersion>> {
    get(&format!("{API}/major_versions?ea=false&ga=true&maintained=true&include_build=false&include_versions=false"))
}

/// The latest package of `major` from `distribution` for this OS and architecture.
pub fn find_package(distribution: &str, major: u32) -> anyhow::Result<Option<Package>> {
    let mut url = format!(
        "{API}/packages?version={major}&distribution={distribution}&operating_system={}&architecture={}\
         &archive_type={}&package_type=jdk&release_status=ga&latest=available&javafx_bundled=false\
         &directly_downloadable=true",
        os(),
        arch(),
        archive_type(),
    );
    if os() == "linux" {
        url.push_str("&lib_c_type=glibc");
    }
    Ok(get::<Package>(&url)?.into_iter().find(|p| p.major_version == major))
}

/// Parses `temurin@21`, `zulu-17` or plain `21` (Temurin).
pub fn parse_spec(spec: &str) -> anyhow::Result<(String, u32)> {
    let spec = spec.trim().to_lowercase();
    let (distro, major) = match spec.rsplit_once(['@', '-', ':']) {
        Some((d, m)) => (d.to_string(), m),
        None => ("temurin".to_string(), spec.as_str()),
    };
    let major = major.parse().with_context(|| format!("'{spec}' is not like temurin@21"))?;
    Ok((distro, major))
}

/// The folder name a package is installed under, e.g. `temurin-21.0.12.1_1`.
pub fn install_name(pkg: &Package) -> String {
    let version: String = pkg
        .java_version
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    format!("{}-{}", pkg.distribution, version)
}

/// Downloads, verifies and extracts a package into `~/.jman/jdks`.
pub fn install(pkg: &Package, mut progress: impl FnMut(Progress)) -> anyhow::Result<Jdk> {
    let jdks = paths::jdks_dir();
    fs::create_dir_all(&jdks)?;
    let dest = jdks.join(install_name(pkg));
    if dest.exists() {
        bail!("{} is already installed", install_name(pkg));
    }

    let info = get::<PackageInfo>(&format!("{API}/ids/{}", pkg.id))?
        .into_iter()
        .next()
        .context("the package has no download information")?;

    let archive = jdks.join(format!(".{}.part", pkg.id));
    let extract_dir = jdks.join(format!(".{}.extract", pkg.id));
    let result = (|| {
        let digest = download(&info.direct_download_uri, &archive, pkg.size, &mut progress)?;
        if info.checksum_type.eq_ignore_ascii_case("sha256") && !info.checksum.is_empty() {
            progress(Progress::Verifying);
            if !digest.eq_ignore_ascii_case(info.checksum.trim()) {
                bail!("checksum mismatch for {}", pkg.filename);
            }
        }
        progress(Progress::Extracting);
        let _ = fs::remove_dir_all(&extract_dir);
        extract(&archive, &extract_dir, &pkg.archive_type)?;
        fs::rename(single_root(&extract_dir)?, &dest).context("moving the extracted JDK into place")?;
        Ok(())
    })();
    let _ = fs::remove_file(&archive);
    let _ = fs::remove_dir_all(&extract_dir);
    result?;

    [dest.clone(), dest.join("Contents").join("Home")]
        .iter()
        .find_map(|home| detect::probe(home, true))
        .context("the downloaded archive does not contain a JDK")
}

/// Streams `url` to `file`, returning its SHA-256 as hex.
fn download(url: &str, file: &Path, size_hint: u64, progress: &mut impl FnMut(Progress)) -> anyhow::Result<String> {
    let res = ureq::get(url)
        .header("User-Agent", concat!("jman/", env!("CARGO_PKG_VERSION")))
        .call()
        .with_context(|| format!("downloading {url}"))?;
    let total = res.body().content_length().unwrap_or(size_hint);
    let mut reader = res.into_body().into_reader();
    let mut out = File::create(file)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0; 256 * 1024];
    let (mut done, mut reported) = (0u64, 0u64);
    progress(Progress::Downloading { done, total });
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if done - reported >= 1 << 20 {
            reported = done;
            progress(Progress::Downloading { done, total });
        }
    }
    progress(Progress::Downloading { done, total: total.max(done) });
    out.flush()?;
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn extract(archive: &Path, dest: &Path, archive_type: &str) -> anyhow::Result<()> {
    fs::create_dir_all(dest)?;
    let file = BufReader::new(File::open(archive)?);
    match archive_type {
        "zip" => zip::ZipArchive::new(file)?.extract(dest)?,
        "tar.gz" | "tgz" => tar::Archive::new(flate2::read::GzDecoder::new(file)).unpack(dest)?,
        other => bail!("unsupported archive type {other}"),
    }
    Ok(())
}

/// JDK archives wrap everything in one top-level folder; return it.
fn single_root(dir: &Path) -> anyhow::Result<PathBuf> {
    let entries: Vec<PathBuf> = fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("._")))
        .collect();
    match entries.as_slice() {
        [only] if only.is_dir() => Ok(only.clone()),
        _ => Ok(dir.to_path_buf()),
    }
}

/// Deletes a JDK that jman installed. Refuses JDKs installed by something else.
pub fn remove(jdk: &Jdk) -> anyhow::Result<()> {
    let jdks = detect::canonical_key(&paths::jdks_dir());
    let home = detect::canonical_key(&jdk.home);
    // the folder directly under ~/.jman/jdks that contains the home (macOS: X/Contents/Home)
    let top = home
        .ancestors()
        .find(|p| p.parent() == Some(jdks.as_path()))
        .with_context(|| format!("{} was not installed by jman; uninstall it with its own installer", jdk.id))?
        .to_path_buf();
    if link::current_target().is_some_and(|t| detect::canonical_key(&t) == home) {
        bail!("{} is the selected JDK; switch to another one first", jdk.id);
    }
    fs::remove_dir_all(&top).or_else(|e| if e.kind() == io::ErrorKind::NotFound { Ok(()) } else { Err(e) })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_specs() {
        assert_eq!(parse_spec("21").unwrap(), ("temurin".into(), 21));
        assert_eq!(parse_spec("zulu@17").unwrap(), ("zulu".into(), 17));
        assert_eq!(parse_spec("Corretto-11").unwrap(), ("corretto".into(), 11));
        assert_eq!(parse_spec("graalvm_community@25").unwrap(), ("graalvm_community".into(), 25));
        assert!(parse_spec("temurin").is_err());
    }

    #[test]
    fn install_names_are_path_safe() {
        let pkg = Package {
            id: "x".into(),
            distribution: "temurin".into(),
            major_version: 21,
            java_version: "21.0.12.1+1".into(),
            archive_type: "zip".into(),
            filename: "f.zip".into(),
            size: 0,
        };
        assert_eq!(install_name(&pkg), "temurin-21.0.12.1_1");
    }
}
