use anyhow::{ensure, Result};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::Path;

/// Ordinary developer/macOS environment. HOME and the final profile variables
/// are set explicitly below; all other variables are omitted unless opted in.
fn default_allowed(key: &str) -> bool {
    matches!(
        key,
        "PATH"
            | "USER"
            | "LOGNAME"
            | "SHELL"
            | "LANG"
            | "TZ"
            | "TMPDIR"
            | "__CF_USER_TEXT_ENCODING"
            | "SSH_AUTH_SOCK"
            | "SSH_AGENT_PID"
            | "SSH_SOCKET_DIR"
            | "DOCKER_HOST"
            | "DOCKER_CONTEXT"
            | "DOCKER_CONFIG"
            | "DOCKER_TLS_VERIFY"
            | "DOCKER_CERT_PATH"
            | "CARGO_HOME"
            | "RUSTUP_HOME"
            | "RUSTUP_DIST_SERVER"
            | "RUSTUP_UPDATE_ROOT"
            | "GOPATH"
            | "GOROOT"
            | "JAVA_HOME"
            | "SDKROOT"
            | "DEVELOPER_DIR"
            | "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "ALL_PROXY"
            | "NO_PROXY"
            | "http_proxy"
            | "https_proxy"
            | "all_proxy"
            | "no_proxy"
    ) || key.starts_with("LC_")
}

pub fn validate_extra_key(key: &str) -> Result<()> {
    let bytes = key.as_bytes();
    ensure!(
        !bytes.is_empty()
            && (bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
            && bytes
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || *b == b'_'),
        "Invalid environment key: {key}"
    );
    ensure!(
        !matches!(
            key,
            "HOME" | "PWD" | "OLDPWD" | "BASH_ENV" | "ENV" | "NODE_OPTIONS"
        ) && !["CODEX_", "HARBOR_", "DYLD_", "ELECTRON_"]
            .iter()
            .any(|p| key.starts_with(p)),
        "Reserved or injection-sensitive environment variable: {key}"
    );
    Ok(())
}

pub fn for_profile(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    home: &Path,
    codex_home: &Path,
    gui_home: &Path,
    cwd: &Path,
    extras: &[String],
) -> Result<BTreeMap<OsString, OsString>> {
    for key in extras {
        validate_extra_key(key)?;
    }
    let mut vars = BTreeMap::new();
    for (key, value) in inherited {
        let allowed = key
            .to_str()
            .is_some_and(|key| default_allowed(key) || extras.iter().any(|extra| extra == key));
        if allowed {
            vars.insert(key, value);
        }
    }
    if !vars.contains_key(OsStr::new("PATH")) {
        vars.insert(
            "PATH".into(),
            "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".into(),
        );
    }
    for (key, value) in [
        ("HOME", home.as_os_str()),
        ("PWD", cwd.as_os_str()),
        ("CODEX_HOME", codex_home.as_os_str()),
        ("CODEX_ELECTRON_USER_DATA_PATH", gui_home.as_os_str()),
        ("CODEX_SPARKLE_ENABLED", OsStr::new("false")),
    ] {
        vars.insert(key.into(), value.to_os_string());
    }
    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_ssh_docker_and_home_but_drops_stale_codex_and_secrets() {
        let input = [
            ("CODEX_HOME", "/old"),
            ("CODEX_SQLITE_HOME", "/old-db"),
            ("OPENAI_API_KEY", "secret"),
            ("MCP_TOKEN", "secret"),
            ("SSH_AUTH_SOCK", "/ssh"),
            ("DOCKER_HOST", "unix:///docker"),
            ("DYLD_INSERT_LIBRARIES", "/evil"),
            ("PATH", "/my/tools"),
        ]
        .map(|(a, b)| (a.into(), b.into()));
        let vars = for_profile(
            input,
            Path::new("/home/user"),
            Path::new("/new/codex"),
            Path::new("/new/gui"),
            Path::new("/cwd"),
            &[],
        )
        .unwrap();
        assert_eq!(vars.get(OsStr::new("HOME")).unwrap(), "/home/user");
        assert_eq!(vars.get(OsStr::new("CODEX_HOME")).unwrap(), "/new/codex");
        assert_eq!(vars.get(OsStr::new("SSH_AUTH_SOCK")).unwrap(), "/ssh");
        assert_eq!(
            vars.get(OsStr::new("DOCKER_HOST")).unwrap(),
            "unix:///docker"
        );
        for key in [
            "OPENAI_API_KEY",
            "MCP_TOKEN",
            "DYLD_INSERT_LIBRARIES",
            "CODEX_SQLITE_HOME",
        ] {
            assert!(!vars.contains_key(OsStr::new(key)));
        }
    }
    #[test]
    fn additional_secret_requires_explicit_name() {
        let vars = for_profile(
            [(OsString::from("WORK_MCP_TOKEN"), OsString::from("secret"))],
            Path::new("/home"),
            Path::new("/codex"),
            Path::new("/gui"),
            Path::new("/home"),
            &["WORK_MCP_TOKEN".into()],
        )
        .unwrap();
        assert!(vars.contains_key(OsStr::new("WORK_MCP_TOKEN")));
    }
    #[test]
    fn reserved_environment_overrides_are_rejected() {
        for key in [
            "HOME",
            "CODEX_HOME",
            "CODEX_SQLITE_HOME",
            "DYLD_INSERT_LIBRARIES",
            "A=B",
            "9KEY",
        ] {
            assert!(validate_extra_key(key).is_err());
        }
    }
}
