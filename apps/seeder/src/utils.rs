use std::process::Command;

pub fn get_root_path() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    output
        .stderr
        .is_empty()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .ok_or_else(|| "Failed to get root path".into())
}
