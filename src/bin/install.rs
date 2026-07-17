use anyhow::Context;
use anyhow::Result;
use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

const INSTALL_PROMPT: &str = "Do you wish to install Orpheus? (Disclaimer: The current installer builds the project from source, so it'll take a while)";
const FINISH_MESSAGE: &str = "Orpheus finished installing successfully!";

// todo: maybe make more os-specific stuff using #[cfg(target_os = "x")]
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

fn get_paths() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let orpheus_bin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/orpheus");

    // todo: add an option to specify a path
    let Some(dest_bin_dir) = dirs::executable_dir() else {
        anyhow::bail!("Failed to find a destination path for binaries.");
    };
    if !dest_bin_dir.exists() && fs::create_dir_all(&dest_bin_dir).is_err() {
        anyhow::bail!("Could not create destination directory for the binary.");
    }

    // todo: add an option to specify a path or to not create a shortcut
    let Some(shortcut_dir) = dirs::data_local_dir().map(|d| d.join("applications")) else {
        anyhow::bail!("Failed to find a destinarion path for shorcuts.");
    };
    if !shortcut_dir.exists() && fs::create_dir_all(&shortcut_dir).is_err() {
        anyhow::bail!("Could not create destination directory for shortcuts.");
    }

    Ok((orpheus_bin_path, dest_bin_dir, shortcut_dir))
}

fn make_files(orpheus_bin_path: PathBuf, dest_bin_dir: &Path, shortcut_dir: &Path) -> Result<()> {
    let dest_bin_path = dest_bin_dir.join("orpheus");
    fs::copy(orpheus_bin_path, &dest_bin_path)
        .context("Failed to move binary to binaries directory.")?;

    let desktop_file = make_desktop_file(&dest_bin_path.to_string_lossy());
    fs::write(shortcut_dir.join("orpheus.desktop"), desktop_file)
        .context("Failed to move shortcut to shortcuts directory.")?;

    let mut perms = fs::metadata(&dest_bin_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dest_bin_path, perms)?;

    Ok(())
}

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

fn make_desktop_file(path: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName=Orpheus\nExec={path}\nTerminal=false")
}

#[allow(dead_code)]
fn get_executable_extension() -> String {
    if cfg!(target_os = "windows") {
        String::from(".exe")
    } else {
        String::new()
    }
}

#[allow(dead_code)]
fn get_shortcut_extension() -> String {
    if cfg!(target_os = "windows") {
        String::from(".lnk")
    } else if cfg!(target_os = "linux") {
        String::from(".desktop")
    } else {
        String::new()
    }
}
