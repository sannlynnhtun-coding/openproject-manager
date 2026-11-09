use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

pub const SERVICES: [&str; 3] = [
    "OpenProjectPostgreSQL",
    "OpenProjectWorker",
    "OpenProjectWeb",
];

pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    cmd
}

fn checked(mut cmd: Command) -> Result<String> {
    let output = cmd.output().context("Could not start Windows command")?;
    let text = output_text(&output);
    if !output.status.success() {
        bail!("{text}");
    }
    Ok(text)
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

pub fn settings_path() -> PathBuf {
    PathBuf::from(std::env::var_os("PROGRAMDATA").unwrap_or_else(|| r"C:\ProgramData".into()))
        .join("OpenProjectManager/manager.json")
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Settings {
    pub folder: PathBuf,
    pub host: String,
    pub port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        let root = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));
        Self {
            folder: root
                .filter(|p| p.join("setup-native.ps1").is_file())
                .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\OpenProject Manager")),
            host: "127.0.0.1".into(),
            port: 8088,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        self.host
            .parse::<Ipv4Addr>()
            .context("Host must be an IPv4 address")?;
        if self.port == 0 {
            bail!("Port must be between 1 and 65535");
        }
        if !self.folder.is_absolute() {
            bail!("Choose an absolute installation folder");
        }
        for file in [
            "setup-native.ps1",
            "runtime/ruby/bin/ruby.exe",
            "runtime/postgresql/bin/pg_ctl.exe",
            "openproject/Gemfile.lock",
        ] {
            if !self.folder.join(file).is_file() {
                bail!(
                    "Missing {file}. Choose the complete OpenProject Manager installation folder."
                );
            }
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        self.validate()?;
        for name in SERVICES {
            if !matches!(state(name)?.as_str(), "Stopped" | "Not installed") {
                bail!("Stop all OpenProject services before saving settings");
            }
        }
        if state(SERVICES[2])? != "Not installed" && installed_folder()? != self.folder {
            bail!("Remove the stopped services before changing installation folders");
        }
        write_json(&settings_path(), self)?;
        let mut cmd = command("icacls.exe");
        cmd.arg(settings_path()).args(["/grant", "*S-1-5-19:R"]);
        checked(cmd)?;
        Ok(())
    }

    pub fn install(&self) -> Result<String> {
        self.save()?;
        setup_bundled(&self.folder)
    }

    pub fn url(&self) -> String {
        format!(
            "http://{}:{}",
            if self.host == "0.0.0.0" {
                "127.0.0.1"
            } else {
                &self.host
            },
            self.port
        )
    }
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::create_dir_all(path.parent().context("Missing settings directory")?)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path).context("Could not save settings")
}

pub fn load() -> Result<Settings> {
    match fs::read(settings_path()) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn installed_folder() -> Result<PathBuf> {
    let mut cmd = command("sc.exe");
    cmd.args(["qc", SERVICES[2]]);
    folder_from_service_config(&checked(cmd)?)
}

pub fn folder_from_service_config(config: &str) -> Result<PathBuf> {
    let suffix = r"\services\openproject-web.exe";
    let executable = service_executable(config)?;
    if !executable.to_ascii_lowercase().ends_with(suffix) {
        bail!("Unexpected service executable");
    }
    let folder = PathBuf::from(&executable[..executable.len() - suffix.len()]);
    if !folder.is_absolute() {
        bail!("Service executable must be absolute");
    }
    Ok(folder)
}

fn service_executable(config: &str) -> Result<String> {
    let line = config
        .lines()
        .find(|line| line.trim_start().starts_with("BINARY_PATH_NAME"))
        .context("Service is not an OpenProject Manager service")?;
    let value = line
        .split_once(':')
        .context("Missing service executable")?
        .1
        .trim()
        .replace('/', "\\");
    let executable = if let Some(quoted) = value.strip_prefix('"') {
        quoted
            .split_once('"')
            .context("Unclosed service executable quote")?
            .0
    } else {
        let end = value
            .to_ascii_lowercase()
            .find(".exe")
            .context("Missing service executable")?
            + 4;
        &value[..end]
    };
    Ok(executable.to_owned())
}

pub fn managed_settings() -> Result<Settings> {
    let mut settings = load()?;
    if state(SERVICES[2])? != "Not installed" {
        settings.folder = installed_folder()?;
    }
    Ok(settings)
}

pub fn attach_existing() -> Result<Settings> {
    let settings = managed_settings()?;
    if state(SERVICES[2])? == "Not installed" {
        bail!("OpenProject Manager services are not installed");
    }
    settings.validate()?;
    Ok(settings)
}

fn state(name: &str) -> Result<String> {
    let output = command("sc.exe").args(["query", name]).output()?;
    if output.status.code() == Some(1060) {
        return Ok("Not installed".into());
    }
    if !output.status.success() {
        bail!("{}", output_text(&output));
    }
    parse_state(&output_text(&output))
}

fn parse_state(text: &str) -> Result<String> {
    let number = text
        .lines()
        .find(|l| l.trim_start().starts_with("STATE"))
        .and_then(|l| l.split_once(':'))
        .and_then(|(_, s)| s.split_whitespace().next())
        .context("Cannot read Windows service state")?;
    Ok(match number {
        "1" => "Stopped",
        "4" => "Running",
        _ => "Starting or stopping",
    }
    .into())
}

pub fn service_status() -> Result<String> {
    let states: Vec<_> = SERVICES
        .iter()
        .map(|name| state(name))
        .collect::<Result<_>>()?;
    for common in ["Not installed", "Running", "Stopped"] {
        if states.iter().all(|s| s == common) {
            return Ok(common.into());
        }
    }
    Ok(if states.iter().any(|s| s == "Not installed") {
        "Incomplete"
    } else if states.iter().any(|s| s == "Starting or stopping") {
        "Starting or stopping"
    } else {
        "Partially running"
    }
    .into())
}

fn verify_owner(settings: &Settings) -> Result<()> {
    for (index, name) in SERVICES.iter().enumerate() {
        if state(name)? == "Not installed" {
            continue;
        }
        let executable = if index == 0 {
            settings.folder.join("runtime/postgresql/bin/pg_ctl.exe")
        } else {
            settings.folder.join(format!(
                "services/{}.exe",
                if index == 1 {
                    "openproject-worker"
                } else {
                    "openproject-web"
                }
            ))
        };
        let mut cmd = command("sc.exe");
        cmd.args(["qc", name]);
        let registered = service_executable(&checked(cmd)?)?;
        let expected = executable
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        if !registered.eq_ignore_ascii_case(&expected) {
            bail!("{name} belongs to a different installation. It was not changed.");
        }
    }
    Ok(())
}

fn wait_for(name: &str, expected: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(100);
    loop {
        if state(name)? == expected {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("Timed out waiting for {name}: {expected}. Check View log.");
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

pub fn control(action: &str) -> Result<String> {
    if !matches!(action, "start" | "stop" | "delete") {
        bail!("Unsupported service action");
    }
    let settings = managed_settings()?;
    verify_owner(&settings)?;
    if action == "start" && state(SERVICES[2])? != "Running" {
        let address: Ipv4Addr = settings.host.parse()?;
        let _listener = std::net::TcpListener::bind((address, settings.port))
            .context("Configured web address or port is unavailable")?;
    }
    if action == "delete" {
        for name in SERVICES {
            if !matches!(state(name)?.as_str(), "Stopped" | "Not installed") {
                bail!("Stop all services before removing them");
            }
        }
    }
    let mut names = SERVICES.to_vec();
    if action != "start" {
        names.reverse();
    }
    for name in names {
        let current = state(name)?;
        if current == "Not installed" {
            if action != "start" {
                continue;
            }
            bail!("{name} is not installed. Repair the installation.");
        }
        if (action == "start" && current == "Running") || (action == "stop" && current == "Stopped")
        {
            continue;
        }
        let mut cmd = command("sc.exe");
        cmd.args([action, name]);
        checked(cmd)?;
        if action != "delete" {
            wait_for(
                name,
                if action == "start" {
                    "Running"
                } else {
                    "Stopped"
                },
            )?;
        }
    }
    Ok(if action == "delete" {
        "Services removed. Database, attachments and credentials were kept."
    } else if action == "start" {
        "Services started. Wait for Web: Ready."
    } else {
        "OpenProject services stopped."
    }
    .into())
}

pub fn restart() -> Result<String> {
    control("stop")?;
    control("start")
}

pub fn setup_bundled(root: &Path) -> Result<String> {
    let mut cmd = command("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ])
    .arg(root.join("setup-native.ps1"))
    .arg("-Root")
    .arg(root);
    checked(cmd)
        .context("OpenProject setup failed. See installer.log; existing data was preserved.")
}

pub fn web_status(settings: &Settings) -> Result<String> {
    let ip: Ipv4Addr = if settings.host == "0.0.0.0" {
        Ipv4Addr::LOCALHOST
    } else {
        settings.host.parse()?
    };
    let mut stream =
        TcpStream::connect_timeout(&(ip, settings.port).into(), Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(4)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    write!(
        stream,
        "GET /login HTTP/1.0\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        ip, settings.port
    )?;
    let mut response = String::new();
    stream.take(512 * 1024).read_to_string(&mut response)?;
    Ok(
        if response.lines().next().is_some_and(|l| l.contains(" 200 "))
            && response.contains("OpenProject")
        {
            "Web: Ready"
        } else {
            "Web: Starting"
        }
        .into(),
    )
}

pub fn read_log(_folder: &Path) -> Result<String> {
    let root = settings_path()
        .parent()
        .context("Missing log folder")?
        .join("logs");
    let mut text = String::new();
    for name in [
        "openproject-web.out.log",
        "openproject-web.err.log",
        "openproject-worker.out.log",
        "openproject-worker.err.log",
        "setup.log",
    ] {
        let path = root.join(name);
        match fs::read_to_string(&path) {
            Ok(content) => {
                text.push_str(&format!("\n{name}\n"));
                let lines: Vec<_> = content.lines().rev().take(60).collect();
                for line in lines.iter().rev() {
                    text.push_str(line);
                    text.push('\n');
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    if text.is_empty() {
        text = "No service logs yet. Run the installer first.".into();
    }
    Ok(text)
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    #[default]
    Dark,
}
pub fn load_appearance() -> Result<Appearance> {
    preference("appearance.json")
}
pub fn save_appearance(value: Appearance) -> Result<()> {
    write_json(&settings_path().with_file_name("appearance.json"), &value)
}
pub fn load_language() -> Result<bool> {
    preference("language.json")
}
pub fn save_language(value: bool) -> Result<()> {
    write_json(&settings_path().with_file_name("language.json"), &value)
}
fn preference<T: serde::de::DeserializeOwned + Default>(name: &str) -> Result<T> {
    match fs::read(settings_path().with_file_name(name)) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_config_handles_spaces_and_rejects_foreign_services() {
        let root = folder_from_service_config("BINARY_PATH_NAME : \"C:\\Program Files\\OpenProject Manager\\services\\openproject-web.exe\"").unwrap();
        assert_eq!(root, PathBuf::from(r"C:\Program Files\OpenProject Manager"));
        assert!(folder_from_service_config("BINARY_PATH_NAME : C:\\other.exe").is_err());
        assert!(folder_from_service_config("BINARY_PATH_NAME : \"C:\\other.exe\" \"C:\\Program Files\\OpenProject Manager\\services\\openproject-web.exe\"").is_err());
        assert!(
            folder_from_service_config(
                "BINARY_PATH_NAME : \"C:\\services\\openproject-web.exe.bak\""
            )
            .is_err()
        );
    }
    #[test]
    fn numeric_service_states_and_local_browser_url() {
        assert_eq!(parse_state("STATE : 4 RUNNING").unwrap(), "Running");
        assert_eq!(parse_state("STATE : 1 STOPPED").unwrap(), "Stopped");
        assert!(parse_state("unrecognized output").is_err());
        let settings = Settings {
            folder: PathBuf::new(),
            host: "0.0.0.0".into(),
            port: 8087,
        };
        assert_eq!(settings.url(), "http://127.0.0.1:8087");
    }
    #[test]
    fn invalid_settings_do_not_write_configuration() {
        let settings = Settings {
            folder: PathBuf::new(),
            host: "127.0.0.1 & calc".into(),
            port: 8087,
        };
        assert!(settings.validate().is_err());
        assert!(
            Settings {
                host: "127.0.0.1".into(),
                port: 0,
                ..settings
            }
            .validate()
            .is_err()
        );
    }
}
