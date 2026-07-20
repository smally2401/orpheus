//! Standalone installer binary (run via `cargo run --bin install` for now,
//! proper releases are planned for later): builds Orpheus in release
//! mode, then copies the binary and creates a shortcute/launcher entry in
//! standard per-platform locations.
//!
//! Platform-specific behaviour (shortcut format, install directories,
//! executable permissions) is split into small `#[cfg(unix)]`/
//! `#[cfg(windows)]` gated function pairs sharing one name (see
//! `make_shortcut` and `get_shortcut_diw`) so the platform-agnostic
//! code (`main`, `get_paths`, `make_files`) doesn't need to know which
//! OS it's running on. macOS isn't supported yet as there's no
//! `cfg(target_os = "macos")` branch for either function, so this
//! currently only builds for Linux and Windows.

use anyhow::Context;
use anyhow::Result;
use std::fs;
use std::io;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

const INSTALL_PROMPT: &str = "Do you wish to install Orpheus? (Disclaimer: The current installer builds the project from source, so it'll take a while)";
const FINISH_MESSAGE: &str = "Orpheus finished installing successfully!";

fn main() -> Result<()> {
    if !ask_confirm(INSTALL_PROMPT) {
        return Ok(());
    }

    build()?;

    let (orpheus_bin_path, dest_bin_dir, shortcut_dir) = get_paths()?;

    make_files(orpheus_bin_path, &dest_bin_dir, &shortcut_dir)?;

    println!("{FINISH_MESSAGE}");

    check_path(&dest_bin_dir);

    Ok(())
}

/// Runs `cargo build --release` for the main Orpheus binary, failing with
/// a descriptive error if the build itself couldn't even start (e.g.
/// `cargo` not found) or if it ran but reported failure.
fn build() -> Result<()> {
    let build = match std::process::Command::new("cargo")
        .args(["build", "--release"])
        .status()
    {
        Ok(value) => value,
        Err(e) => {
            anyhow::bail!("An error happened while building: {e}");
        }
    };

    if !build.success() {
        anyhow::bail!("An error happened while building.");
    }

    Ok(())
}

/// Resolves where the freshly-build binary lives, where it should be
/// installed to, and where its shortcut should go, creating those
/// destination directories if they don't already exist.
///
/// The shortcut directory is platform-specific (see `get_shortcut_dir`),
/// everything else here is shared across platforms.
fn get_paths() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let orpheus_bin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release")
        .join(format!("orpheus{}", get_executable_extension()));

    // todo: add an option to specify a path
    let Some(dest_bin_dir) = dirs::executable_dir() else {
        anyhow::bail!("Failed to find a destination path for binaries.");
    };
    if !dest_bin_dir.exists() && fs::create_dir_all(&dest_bin_dir).is_err() {
        anyhow::bail!("Could not create destination directory for the binary.");
    }

    // todo: add an option to specify a path or to not create a shortcut
    let shortcut_dir = get_shortcut_dir()?;

    Ok((orpheus_bin_path, dest_bin_dir, shortcut_dir))
}

/// Copies the built binary into place, creates its shortcut, and (on
/// Unix) marks it executable.
///
/// `make_shortcut` is called unconditionally here regardless of platform:
/// it resolves to whichever `#[cfg]` gated implementation matches the
/// build target, so this function doesn't need its own platform branch
/// for that part. The permission setting block is Unix-only since
/// Windows has no equivalent concept of a POSIX executable bit.
fn make_files(orpheus_bin_path: PathBuf, dest_bin_dir: &Path, shortcut_dir: &Path) -> Result<()> {
    let dest_bin_path = dest_bin_dir.join(format!("orpheus{}", get_executable_extension()));
    fs::copy(orpheus_bin_path, &dest_bin_path)
        .context("Failed to move binary to binaries directory.")?;

    make_shortcut(&dest_bin_path, shortcut_dir)?;

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&dest_bin_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_bin_path, perms)?;
    }

    Ok(())
}

/// Prompts the user with a yes/no question on stdin, re-prompting on any
/// unrecognized input rather than defaulting to yes or no.
fn ask_confirm(prompt: &str) -> bool {
    loop {
        print!("{prompt} [y/n]: ");

        if let Err(e) = io::stdout().flush() {
            eprintln!("Failed to flush stdout: {e}");
            continue;
        }

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read input. Please try again.");
            continue;
        }

        let trimmed = input.trim().to_lowercase();

        match trimmed.as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => {
                println!("Invalid input. Please enter 'y' or 'n'.");
            }
        }
    }
}

/// Warns the user if the binary's install directory isn't on their
/// `PATH`, since otherwise they'd have installed successfully but still
/// be unable to run `orpheus` from a terminal without the full path.
fn check_path(dest_bin_dir: &Path) {
    let in_path = std::env::var("PATH")
        .map(|p| std::env::split_paths(&p).any(|entry| entry == dest_bin_dir))
        .unwrap_or(false);

    if !in_path {
        println!(
            "Note: {} is not on your PATH. To run 'orpheus' from a terminal, add this to your shell config:\n   export PATH=\"{}:$PATH\"",
            dest_bin_dir.display(),
            dest_bin_dir.display(),
        );
    }
}

/// Builds the contents of a Linux `.desktop` shortcut file pointing at
/// the installed binary. Only ever called from the Unix `make_shortcut`,
/// but kept unconditional since it's just string formatting with no
/// platform specific APIs involved.
fn make_desktop_file(path: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName=Orpheus\nExec={path}\nTerminal=false")
}

/// The platform appropiate binary extension: `.exe` on Windows, none
/// elsewhere.
fn get_executable_extension() -> String {
    if cfg!(target_os = "windows") {
        String::from(".exe")
    } else {
        String::new()
    }
}

/// Writes a Linux `.desktop` shortcut file pointing at the installed
/// binary.
#[cfg(unix)]
fn make_shortcut(orpheus_bin_path: &Path, shortcut_dir: &Path) -> Result<()> {
    let desktop_file = make_desktop_file(&orpheus_bin_path.to_string_lossy());
    fs::write(shortcut_dir.join("orpheus.desktop"), desktop_file)
        .context("Failed to move shortcut to shortcuts directory.")?;
    Ok(())
}

/// Writes a Windows `.lnk` shortcut pointing at the installed binary,
/// via the `mslnk` crate (there's no plain-text shortcut format to
/// hand-write on Windows the way there is with `.desktop` on Linux).
///
/// Untested against a real Windows build as of writing.
#[cfg(windows)]
fn make_shortcut(orpheus_bin_path: &Path, shortcut_dir: &Path) -> Result<()> {
    use mslnk::ShellLink;

    let sl = ShellLink::new(orpheus_bin_path).context("Failed to build Windows shortcut")?;
    sl.create_lnk(shortcut_dir.join("Orpheus.lnk"))
        .context("Failed to write Windows shortcut")?;
    Ok(())
}

/// Linux shortcut destination: the standard XDG applications directory,
/// so the shortcut shows up in application launchers/menus.
#[cfg(unix)]
fn get_shortcut_dir() -> Result<PathBuf> {
    let Some(shortcut_dir) = dirs::data_local_dir().map(|d| d.join("applications")) else {
        anyhow::bail!("Failed to find a destinarion path for shorcuts.");
    };
    if !shortcut_dir.exists() && fs::create_dir_all(&shortcut_dir).is_err() {
        anyhow::bail!("Could not create destination directory for shortcuts.");
    }
    Ok(shortcut_dir)
}

/// Windows shortcut destination: the current user's Start Menu Programs
/// folder, so the shortcut shows up in the Start Menu.
#[cfg(windows)]
fn get_shortcut_dir() -> Result<PathBuf> {
    let Some(appdata) = dirs::data_dir() else {
        anyhow::bail!("Failed to find a destination path for shortcuts.");
    };
    let shortcut_dir = appdata
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs");
    if !shortcut_dir.exists() && fs::create_dir_all(&shortcut_dir).is_err() {
        anyhow::bail!("Could not create destination directory for shortcuts.");
    }
    Ok(shortcut_dir)
}
