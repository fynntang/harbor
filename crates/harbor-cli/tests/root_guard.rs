use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DEFAULT_DATA: [&str; 4] = [
    ".codex",
    "Library/Application Support/com.openai.codex",
    "Library/Application Support/Codex",
    "Library/Application Support/ChatGPT",
];

struct Fixture {
    temp: tempfile::TempDir,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        fs::create_dir(&home).unwrap();
        Self {
            temp,
            home: home.canonicalize().unwrap(),
        }
    }

    fn list(&self, root: Option<&Path>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_harbor"));
        command.env("HOME", &self.home);
        if let Some(root) = root {
            command.arg("--root").arg(root);
        }
        command.arg("list").output().unwrap()
    }

    fn rejected_without_changes(&self, root: &Path) {
        let before = snapshot(self.temp.path());
        let output = self.list(Some(root));
        assert_eq!(
            snapshot(self.temp.path()),
            before,
            "rejected root changed the filesystem: {}",
            root.display()
        );
        assert!(
            !output.status.success(),
            "accepted protected root {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

// Read only synthetic fixtures; record permissions, contents and link destinations.
fn snapshot(root: &Path) -> Vec<(PathBuf, u32, Vec<u8>)> {
    fn visit(root: &Path, current: &Path, entries: &mut Vec<(PathBuf, u32, Vec<u8>)>) {
        let metadata = fs::symlink_metadata(current).unwrap();
        let content = if metadata.file_type().is_symlink() {
            fs::read_link(current)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else if metadata.is_file() {
            fs::read(current).unwrap()
        } else {
            Vec::new()
        };
        entries.push((
            current.strip_prefix(root).unwrap().to_path_buf(),
            metadata.permissions().mode(),
            content,
        ));
        if metadata.is_dir() {
            for entry in fs::read_dir(current).unwrap() {
                visit(root, &entry.unwrap().path(), entries);
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

fn existing_account(directory: &Path) {
    fs::create_dir_all(directory).unwrap();
    fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        directory.join("sentinel"),
        "unchanged synthetic account data",
    )
    .unwrap();
}

#[test]
fn existing_default_roots_are_rejected_without_changes() {
    for relative in DEFAULT_DATA {
        let fixture = Fixture::new();
        let root = fixture.home.join(relative);
        existing_account(&root);
        fixture.rejected_without_changes(&root);
    }
}

#[test]
fn missing_default_roots_and_descendants_are_not_created() {
    for relative in DEFAULT_DATA {
        let fixture = Fixture::new();
        fixture.rejected_without_changes(&fixture.home.join(relative));
        fixture.rejected_without_changes(&fixture.home.join(relative).join("new/metadata"));
    }
}

#[test]
fn missing_default_case_aliases_are_rejected_without_changes() {
    for relative in [
        ".CODEX",
        "library/application support/COM.OPENAI.CODEX",
        "Library/Application Support/codex",
        "Library/Application Support/chatgpt",
    ] {
        let fixture = Fixture::new();
        fixture.rejected_without_changes(&fixture.home.join(relative).join("new/metadata"));
    }
}

#[test]
fn existing_default_descendants_are_rejected_without_changes() {
    for relative in DEFAULT_DATA {
        let fixture = Fixture::new();
        let root = fixture.home.join(relative).join("metadata");
        existing_account(&root);
        fixture.rejected_without_changes(&root);
    }
}

#[test]
fn default_ancestors_home_and_filesystem_root_are_rejected() {
    let fixture = Fixture::new();
    existing_account(&fixture.home.join("Library/Application Support"));
    for root in [
        fixture.home.join("Library/Application Support"),
        fixture.home.join("Library"),
        fixture.home.clone(),
        PathBuf::from("/"),
    ] {
        fixture.rejected_without_changes(&root);
    }
}

#[test]
fn root_ancestor_symlink_cannot_hide_a_default_descendant() {
    let fixture = Fixture::new();
    let account = fixture.home.join(".codex");
    existing_account(&account);
    let alias = fixture.temp.path().join("account-alias");
    symlink(&account, &alias).unwrap();
    fixture.rejected_without_changes(&alias.join("new/metadata"));
}

#[test]
fn default_ancestor_symlink_is_resolved_with_a_missing_tail() {
    let fixture = Fixture::new();
    let library = fixture.temp.path().join("external-library");
    fs::create_dir(&library).unwrap();
    symlink(&library, fixture.home.join("Library")).unwrap();
    fixture.rejected_without_changes(&library.join("Application Support/Codex/new/metadata"));
}

#[test]
fn default_directory_symlink_protects_its_target_and_target_parent() {
    let fixture = Fixture::new();
    let external = fixture.temp.path().join("external");
    let account = external.join("account");
    existing_account(&account);
    symlink(&account, fixture.home.join(".codex")).unwrap();
    fixture.rejected_without_changes(&account);
    fixture.rejected_without_changes(&external);
    fixture.rejected_without_changes(&account.join("new/metadata"));
}

#[test]
fn unresolved_default_paths_fail_before_creating_a_root() {
    let fixture = Fixture::new();
    symlink(
        fixture.home.join("missing-target"),
        fixture.home.join(".codex"),
    )
    .unwrap();
    fixture.rejected_without_changes(&fixture.home.join("safe-new-root"));
}

#[test]
fn unresolved_root_ancestor_fails_without_changes() {
    let fixture = Fixture::new();
    let alias = fixture.home.join("broken-alias");
    symlink(fixture.home.join("missing-target"), &alias).unwrap();
    fixture.rejected_without_changes(&alias.join("metadata"));
}

#[test]
fn final_root_symlink_is_still_rejected() {
    let fixture = Fixture::new();
    let target = fixture.home.join("safe-target");
    existing_account(&target);
    let alias = fixture.home.join("safe-alias");
    symlink(&target, &alias).unwrap();
    fixture.rejected_without_changes(&alias);
}

#[test]
fn default_and_dedicated_new_roots_remain_usable() {
    let fixture = Fixture::new();
    assert!(fixture.list(None).status.success());
    assert!(fixture
        .home
        .join("Library/Application Support/Harbor/profiles")
        .is_dir());
    let custom = fixture.home.join("custom/new/harbor");
    assert!(fixture.list(Some(&custom)).status.success());
    assert!(custom.join("profiles").is_dir());
    assert_eq!(
        fs::metadata(custom).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

#[test]
fn safe_root_ancestor_symlink_remains_usable() {
    let fixture = Fixture::new();
    let target = fixture.home.join("safe-parent");
    fs::create_dir(&target).unwrap();
    let alias = fixture.home.join("safe-parent-alias");
    symlink(&target, &alias).unwrap();
    assert!(fixture
        .list(Some(&alias.join("new/metadata")))
        .status
        .success());
    assert!(target.join("new/metadata/profiles").is_dir());
}
