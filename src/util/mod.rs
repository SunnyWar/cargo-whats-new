use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn setup_temp_workspace(workspace_root: &str, verbose: bool) -> Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();
    if verbose {
        println!("Created temporary workspace at: {}", temp_path.display());
    }

    let root = Path::new(workspace_root);
    for file in ["Cargo.toml", "Cargo.lock"] {
        let src = root.join(file);
        let dst = temp_path.join(file);
        if src.exists() {
            fs::copy(&src, &dst)?;
            if verbose {
                println!("Copied {} to temp workspace", file);
            }
        } else if verbose {
            println!("Warning: {} not found in workspace root", file);
        }
    }

    // Also copy the src directory if it exists
    let src_dir = root.join("src");
    let dst_src_dir = temp_path.join("src");
    if src_dir.exists() && src_dir.is_dir() {
        copy_dir_all(&src_dir, &dst_src_dir, verbose)?;
        if verbose {
            println!("Copied src directory to temp workspace");
        }
    } else if verbose {
        println!("Warning: src directory not found in workspace root");
    }

    if verbose {
        println!("\n[DEBUG] Files in temp workspace:");
        for entry in fs::read_dir(temp_path)? {
            let entry = entry?;
            let path = entry.path();
            println!("[DEBUG] - {}", path.display());
        }
    }

    Ok(temp_dir)
}

fn copy_dir_all(src: &Path, dst: &Path, verbose: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path, verbose)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
