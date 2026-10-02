//! Double-click launcher for Windows, "Lancer Cent Ans.exe" at the root of the repository
//! (ADR 0153). Same job as "Lancer Cent Ans.bat" (ADR 0117): find the bash of Git for Windows
//! and run `tools/launch.sh`, which rebuilds and reimports what changed, then starts the game.
//! All the launch logic stays in `tools/launch.sh`; arguments are passed through unchanged.

use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const LAUNCH_SCRIPT: &str = "tools/launch.sh";
const GIT_DOWNLOAD_URL: &str = "https://git-scm.com/download/win";

/// Places where the bash of Git for Windows may be, most likely first: machine install, user
/// install, scoop, then next to every `git.exe` of the PATH (`<Git>\cmd\git.exe` →
/// `<Git>\bin\bash.exe`). The bash of WSL (`System32\bash.exe`) is never a candidate: it does
/// not see the Windows toolchain.
fn bash_candidates(
    variable: impl Fn(&str) -> Option<PathBuf>,
    path_directories: &[PathBuf],
) -> Vec<PathBuf> {
    let install_roots = [
        ("ProgramFiles", "Git"),
        ("ProgramW6432", "Git"),
        ("LocalAppData", "Programs/Git"),
        ("UserProfile", "scoop/apps/git/current"),
    ];
    let mut candidates: Vec<PathBuf> = install_roots
        .iter()
        .filter_map(|(name, git_directory)| Some(variable(name)?.join(git_directory)))
        .map(|git_root| git_root.join("bin").join("bash.exe"))
        .collect();
    for directory in path_directories {
        if directory.join("git.exe").is_file() {
            if let Some(git_root) = directory.parent() {
                candidates.push(git_root.join("bin").join("bash.exe"));
            }
        }
    }
    candidates
}

fn find_git_bash() -> Option<PathBuf> {
    let path_directories: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();
    bash_candidates(
        |name| env::var_os(name).map(PathBuf::from),
        &path_directories,
    )
    .into_iter()
    .find(|candidate| candidate.is_file())
}

/// The repository root is the directory of the executable.
fn repository_root() -> Result<PathBuf, String> {
    let executable = env::current_exe()
        .map_err(|error| format!("Emplacement du lanceur introuvable : {error}"))?;
    let root = executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or("Emplacement du lanceur introuvable.")?;
    if !root.join(LAUNCH_SCRIPT).is_file() {
        return Err(format!(
            "{LAUNCH_SCRIPT} introuvable à côté du lanceur : « Lancer Cent Ans.exe » doit rester \
             à la racine du dépôt cloné (git clone https://github.com/BenjaminNavet/cent-ans.git)."
        ));
    }
    Ok(root)
}

/// Runs `tools/launch.sh` in Git Bash and returns its exit code.
fn launch() -> Result<u8, String> {
    let root = repository_root()?;
    let bash = find_git_bash().ok_or(format!(
        "Git pour Windows (Git Bash) est nécessaire : {GIT_DOWNLOAD_URL}"
    ))?;
    let status = Command::new(&bash)
        .arg(LAUNCH_SCRIPT)
        .args(env::args_os().skip(1))
        .current_dir(&root)
        .status()
        .map_err(|error| format!("Impossible d'exécuter {} : {error}", bash.display()))?;
    // A code outside 0..=255 (process killed, Windows status code) still counts as a failure.
    Ok(status
        .code()
        .map_or(1, |code| u8::try_from(code).unwrap_or(1)))
}

/// Keeps the window of a double-click open so that the error can be read. Not in CI (GitHub
/// Actions sets CI): nobody reads the window there.
fn wait_for_enter() {
    if env::var_os("CI").is_some() {
        return;
    }
    print!("Appuyez sur Entrée pour fermer…");
    let _ = io::stdout().flush();
    let _ = io::stdin().lock().read_line(&mut String::new());
}

fn main() -> ExitCode {
    let code = launch().unwrap_or_else(|message| {
        eprintln!("[Cent Ans] {message}");
        1
    });
    if code != 0 {
        wait_for_enter();
    }
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_places_come_before_the_path() {
        let variable = |name: &str| match name {
            "ProgramFiles" => Some(PathBuf::from("C:/Program Files")),
            "LocalAppData" => Some(PathBuf::from("C:/Users/jeanne/AppData/Local")),
            _ => None,
        };
        let candidates = bash_candidates(variable, &[]);
        assert_eq!(
            candidates,
            vec![
                PathBuf::from("C:/Program Files/Git/bin/bash.exe"),
                PathBuf::from("C:/Users/jeanne/AppData/Local/Programs/Git/bin/bash.exe"),
            ]
        );
    }

    #[test]
    fn bash_is_found_next_to_a_git_of_the_path() {
        let scratch = env::temp_dir().join(format!("cent-ans-launcher-{}", std::process::id()));
        let git_cmd = scratch.join("PortableGit").join("cmd");
        let without_git = scratch.join("System32");
        std::fs::create_dir_all(&git_cmd).unwrap();
        std::fs::create_dir_all(&without_git).unwrap();
        std::fs::write(git_cmd.join("git.exe"), b"").unwrap();
        // WSL's bash sits in a PATH directory without git.exe: it must not be proposed.
        std::fs::write(without_git.join("bash.exe"), b"").unwrap();

        let candidates = bash_candidates(|_| None, &[without_git, git_cmd]);

        assert_eq!(
            candidates,
            vec![scratch.join("PortableGit").join("bin").join("bash.exe")]
        );
        std::fs::remove_dir_all(&scratch).unwrap();
    }
}
