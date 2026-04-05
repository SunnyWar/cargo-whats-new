use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn setup_temp_workspace(workspace_root: &str) -> Result<tempfile::TempDir> {
    use std::fs;
    use std::path::Path;
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();
    println!("Created temporary workspace at: {}", temp_path.display());

    let root = Path::new(workspace_root);
    for file in ["Cargo.toml", "Cargo.lock"] {
        let src = root.join(file);
        let dst = temp_path.join(file);
        if src.exists() {
            fs::copy(&src, &dst)?;
            println!("Copied {} to temp workspace", file);
        } else {
            println!("Warning: {} not found in workspace root", file);
        }
    }

    // Debug: List files in temp workspace to confirm step 1 worked
    println!("\n[DEBUG] Files in temp workspace:");
    for entry in fs::read_dir(temp_path)? {
        let entry = entry?;
        let path = entry.path();
        println!("[DEBUG] - {}", path.display());
    }

    Ok(temp_dir)
}

fn main() -> Result<()> {
    // Load metadata for the current workspace
    let metadata = MetadataCommand::new().exec()?;

    println!("Workspace root: {}", metadata.workspace_root);

    // Step 1: Create temp workspace and copy files
    let _temp_dir = setup_temp_workspace(metadata.workspace_root.as_str())?;

    println!("\nPackages:");
    for pkg in metadata.packages {
        println!(
            "- {} {} ({})",
            pkg.name,
            pkg.version,
            pkg.manifest_path.as_str()
        );
    }

    Ok(())
}
