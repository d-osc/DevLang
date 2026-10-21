//! `d update`: install the latest (or a chosen) DevLang release.
//!
//! Releases publish an offline installer per platform plus `SHA256SUMS`. The
//! installer is verified against that manifest and then run against the
//! existing managed installation, so ownership rules stay in one place.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{io::Read, path::PathBuf, process::Command};

const API: &str = "https://api.github.com/repos/d-osc/DevLang/releases";
const DOWNLOADS: &str = "https://github.com/d-osc/DevLang/releases/download/";
const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

const USAGE: &str = "usage: d update [--check] [--version vX.Y.Z]";

pub fn command(args: &[String]) -> Result<i32, String> {
    let mut check = false;
    let mut wanted = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--version" => wanted = Some(it.next().ok_or(USAGE)?.clone()),
            "-h" | "--help" => {
                println!("{USAGE}\n  --check    Report the newest release without installing\n  --version  Install a specific release tag");
                return Ok(0);
            }
            _ => return Err(USAGE.into()),
        }
    }
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    let url = match &wanted {
        Some(tag) => {
            version_of(tag)?;
            format!("{API}/tags/{tag}")
        }
        None => format!("{API}/latest"),
    };
    let release: Release = serde_json::from_slice(&download(&url, 4 * 1024 * 1024)?)
        .map_err(|e| format!("unexpected release response: {e}"))?;
    let latest = version_of(&release.tag_name)?;
    if wanted.is_none() && latest <= current {
        println!("DevLang {current} is up to date (latest release: {})", release.tag_name);
        return Ok(0);
    }
    println!("DevLang {current} -> {}", release.tag_name);
    if check {
        return Ok(0);
    }
    let platform = platform()?;
    let installer = installer_name(&release.tag_name, platform)?;
    let find = |name: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name == name && a.browser_download_url.starts_with(DOWNLOADS))
            .map(|a| a.browser_download_url.clone())
            .ok_or_else(|| format!("release {} has no asset {name}", release.tag_name))
    };
    let sums = String::from_utf8(download(&find("SHA256SUMS")?, 1024 * 1024)?)
        .map_err(|_| "SHA256SUMS is not UTF-8")?;
    let expected = checksum(&sums, &installer)?;
    let bytes = download(&find(&installer)?, MAX_DOWNLOAD)?;
    let actual: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    if actual != expected {
        return Err(format!("checksum mismatch for {installer}; nothing was installed"));
    }
    let directory = std::env::temp_dir().join(format!("devlang-update-{}", std::process::id()));
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let path = directory.join(&installer);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    apply(&path, platform)
}

fn version_of(tag: &str) -> Result<semver::Version, String> {
    tag.strip_prefix('v')
        .and_then(|v| semver::Version::parse(v).ok())
        .ok_or_else(|| format!("'{tag}' is not a release tag such as v0.5.4"))
}

fn platform() -> Result<&'static str, String> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-arm64",
        ("windows", "x86_64") => "windows-x86_64",
        ("windows", "aarch64") => "windows-arm64",
        (os, arch) => return Err(format!("no DevLang release for {os}/{arch}")),
    })
}

fn installer_name(tag: &str, platform: &str) -> Result<String, String> {
    match platform {
        "linux-x86_64" | "linux-arm64" => Ok(format!("devlang-setup-{tag}-{platform}.py")),
        "windows-x86_64" => Ok(format!("devlang-setup-{tag}-{platform}.exe")),
        _ => Err(format!(
            "automatic update is not available for {platform}; download devlang-setup-{tag}-{platform}.zip from {DOWNLOADS}{tag}/ and run install.cmd"
        )),
    }
}

fn checksum(sums: &str, name: &str) -> Result<String, String> {
    sums.lines()
        .filter_map(|l| l.split_once("  "))
        .find(|(_, n)| *n == name)
        .map(|(digest, _)| digest.to_ascii_lowercase())
        .ok_or_else(|| format!("SHA256SUMS does not list {name}"))
}

fn download(url: &str, max: u64) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(300)))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("User-Agent", concat!("devlang-d/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err(format!("download exceeds {max} bytes"));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn apply(installer: &std::path::Path, _platform: &str) -> Result<i32, String> {
    let exe = std::env::current_exe().and_then(|p| p.canonicalize()).map_err(|e| e.to_string())?;
    let prefix: PathBuf = exe.parent().ok_or("cannot find the installation directory")?.to_owned();
    let marker = prefix.join(".devlang-install.json");
    let text = std::fs::read_to_string(&marker).map_err(|_| {
        format!(
            "{} is not a managed installation; run the installer manually: python3 {}",
            prefix.display(),
            installer.display()
        )
    })?;
    let info: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let bin_dir = info["bin_dir"].as_str().ok_or("installation marker has no bin_dir")?;
    let status = Command::new("python3")
        .arg(installer)
        .args(["--prefix"])
        .arg(&prefix)
        .args(["--bin-dir", bin_dir])
        .status()
        .map_err(|e| format!("cannot run python3: {e}"))?;
    let _ = std::fs::remove_dir_all(installer.parent().unwrap());
    Ok(status.code().unwrap_or(1))
}

#[cfg(windows)]
fn apply(installer: &std::path::Path, _platform: &str) -> Result<i32, String> {
    // d.exe cannot be replaced while running: start the installer after exit.
    Command::new("cmd")
        .args(["/C", "timeout /t 2 /nobreak >nul & "])
        .arg(installer)
        .arg("--silent")
        .spawn()
        .map_err(|e| e.to_string())?;
    println!("The installer will finish in the background; open a new terminal afterwards.");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_checksums() {
        assert_eq!(installer_name("v0.5.4", "linux-arm64").unwrap(), "devlang-setup-v0.5.4-linux-arm64.py");
        assert!(installer_name("v0.5.4", "windows-arm64").is_err());
        let sums = "AB12  a.py\ncd34  b.exe\n";
        assert_eq!(checksum(sums, "b.exe").unwrap(), "cd34");
        assert!(checksum(sums, "c").is_err());
        assert!(version_of("0.5.4").is_err() && version_of("v0.5.4").is_ok());
    }
}
