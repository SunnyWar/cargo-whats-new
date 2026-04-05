use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::Path;
use std::process::Command;
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

    // Also copy the src directory if it exists
    let src_dir = root.join("src");
    let dst_src_dir = temp_path.join("src");
    if src_dir.exists() && src_dir.is_dir() {
        // Recursively copy src directory
        fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
            std::fs::create_dir_all(dst)?;
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());
                if file_type.is_dir() {
                    copy_dir_all(&src_path, &dst_path)?;
                } else {
                    std::fs::copy(&src_path, &dst_path)?;
                }
            }
            Ok(())
        }
        copy_dir_all(&src_dir, &dst_src_dir)?;
        println!("Copied src directory to temp workspace");
    } else {
        println!("Warning: src directory not found in workspace root");
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

fn run_cargo_update(temp_path: &std::path::Path) -> Result<()> {
    println!("\n[DEBUG] Running 'cargo update' in temp workspace...");
    let status = Command::new("cargo")
        .arg("update")
        .current_dir(temp_path)
        .status()?;
    if status.success() {
        println!("[DEBUG] 'cargo update' completed successfully.");
        Ok(())
    } else {
        anyhow::bail!("'cargo update' failed in temp workspace");
    }
}

fn load_metadata_from_path(path: &std::path::Path) -> Result<cargo_metadata::Metadata> {
    let mut cmd = MetadataCommand::new();
    cmd.current_dir(path);
    let metadata = cmd.exec()?;
    println!("[DEBUG] Loaded updated dependency graph from temp workspace.");
    Ok(metadata)
}

fn main() -> Result<()> {
    // Load metadata for the current workspace
    let metadata = MetadataCommand::new().exec()?;

    println!("Workspace root: {}", metadata.workspace_root);

    // Step 1: Create temp workspace and copy files
    let temp_dir = setup_temp_workspace(metadata.workspace_root.as_str())?;

    // Step 2: Run cargo update in temp workspace
    run_cargo_update(temp_dir.path())?;

    // Step 3: Load updated dependency graph from temp workspace
    let updated_metadata = load_metadata_from_path(temp_dir.path())?;

    println!("\nPackages (original):");
    for pkg in metadata.packages {
        println!(
            "- {} {} ({})",
            pkg.name,
            pkg.version,
            pkg.manifest_path.as_str()
        );
    }

    println!("\nPackages (after update):");
    for pkg in updated_metadata.packages {
        println!(
            "- {} {} ({})",
            pkg.name,
            pkg.version,
            pkg.manifest_path.as_str()
        );
    }

    Ok(())
}
