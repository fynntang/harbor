use anyhow::{ensure, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub bundle: PathBuf,
    pub executable: PathBuf,
    pub bundle_id: String,
    pub version: String,
    pub build_version: String,
}

impl AppInfo {
    /// Reads both XML and binary plists. Never modifies vendor files.
    pub fn inspect(bundle: &Path) -> Result<Self> {
        let bundle = bundle
            .canonicalize()
            .with_context(|| format!("App not found: {}", bundle.display()))?;
        ensure!(
            bundle.extension().is_some_and(|x| x == "app"),
            "Expected a .app bundle"
        );
        let value = plist::Value::from_file(bundle.join("Contents/Info.plist"))?;
        let dict = value
            .as_dictionary()
            .context("Info.plist is not a dictionary")?;
        let get = |key: &str| -> Result<String> {
            let value = dict
                .get(key)
                .and_then(plist::Value::as_string)
                .with_context(|| format!("Missing string {key} in Info.plist"))?;
            ensure!(!value.trim().is_empty(), "Empty {key}");
            Ok(value.to_owned())
        };
        let executable_name = get("CFBundleExecutable")?;
        let parts: Vec<_> = Path::new(&executable_name).components().collect();
        ensure!(
            parts.len() == 1 && matches!(parts[0], Component::Normal(_)),
            "CFBundleExecutable must be a file name, not a path"
        );
        ensure!(
            matches!(executable_name.as_str(), "ChatGPT" | "Codex"),
            "Harbor supports only the tested ChatGPT/Codex desktop executable"
        );
        ensure!(dict.contains_key("CrProductDirName") || dict.contains_key("ElectronAsarIntegrity"),
            "This bundle does not match the tested Chromium/Codex desktop layout; refusing to assume profile flags work");
        let executable = bundle
            .join("Contents/MacOS")
            .join(executable_name)
            .canonicalize()?;
        ensure!(
            executable.starts_with(&bundle),
            "Executable resolves outside bundle"
        );
        let meta = fs::metadata(&executable)?;
        ensure!(
            meta.is_file() && meta.permissions().mode() & 0o111 != 0,
            "Not an executable file: {}",
            executable.display()
        );
        Ok(Self {
            executable,
            bundle_id: get("CFBundleIdentifier")?,
            version: get("CFBundleShortVersionString")?,
            build_version: get("CFBundleVersion")?,
            bundle,
        })
    }
}
