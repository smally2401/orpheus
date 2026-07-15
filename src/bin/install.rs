use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitStatus;

const INSTALL_PROMPT: &str = "Do you wish to install Orpheus? (Disclaimer: The current installer builds the project from source, so it'll take a while)";

fn main() -> io::Result<()> {

    if !ask_confirm(INSTALL_PROMPT) {
        return Ok(());
    }

    let build = match std::process::Command::new("cargo").args(["build", "--release"]).status() {
        Ok(value) => value,
        Err(e) => {
            println!("An error happened while building: {e}");
            return Ok(());
        },
    };

    if !ExitStatus::success(&build) {
        println!("An error happened while building.");
        return Ok(());
    }

    let orpheus_bin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/orpheus");
    let Some(dest_bin_path) = dirs::executable_dir() else {
        println!("Failed to find a destination path for binaries.");
        return Ok(());
    };
    let shortcut_path = dirs::data_local_dir();




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

fn desktop_file(path: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName=Orpheus\nExec={path}")
}
