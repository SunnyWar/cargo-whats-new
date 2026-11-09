use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn setup_temp_workspace(
    workspace_root: &str,
    member_dirs: &[PathBuf],
    verbose: bool,
) -> Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();
    if verbose {
        println!("Created temporary workspace at: {}", temp_path.display());
    }

    let root = Path::new(workspace_root);
    copy_manifest_and_src(root, temp_path, true, verbose)?;

    for member_dir in member_dirs {
        let member_src = root.join(member_dir);
        let member_dst = temp_path.join(member_dir);
        fs::create_dir_all(&member_dst)?;
        copy_manifest_and_src(&member_src, &member_dst, false, verbose)?;
    }

    if verbose {
        #[cfg(debug_assertions)]
        {
            println!("\n[DEBUG] Files in temp workspace:");
            for entry in fs::read_dir(temp_path)? {
                let entry = entry?;
                let path = entry.path();
                println!("[DEBUG] - {}", path.display());
            }
        }
    }

    Ok(temp_dir)
}

fn copy_manifest_and_src(
    src_root: &Path,
    dst_root: &Path,
    copy_lock: bool,
    verbose: bool,
) -> Result<()> {
    let manifest_src = src_root.join("Cargo.toml");
    let manifest_dst = dst_root.join("Cargo.toml");
    if manifest_src.exists() {
        fs::copy(&manifest_src, &manifest_dst)?;
        if verbose {
            println!("Copied {} to temp workspace", manifest_src.display());
        }
    } else if verbose {
        println!("Warning: Cargo.toml not found in {}", src_root.display());
    }

    if copy_lock {
        let lock_src = src_root.join("Cargo.lock");
        let lock_dst = dst_root.join("Cargo.lock");
        if lock_src.exists() {
            fs::copy(&lock_src, &lock_dst)?;
            if verbose {
                println!("Copied {} to temp workspace", lock_src.display());
            }
        } else if verbose {
            println!("Warning: Cargo.lock not found in {}", src_root.display());
        }
    }

    let src_dir = src_root.join("src");
    let dst_src_dir = dst_root.join("src");
    if src_dir.exists() && src_dir.is_dir() {
        copy_dir_all(&src_dir, &dst_src_dir, verbose)?;
        if verbose {
            println!("Copied {} to temp workspace", src_dir.display());
        }
    } else if verbose {
        println!("Warning: src directory not found in {}", src_root.display());
    }

    Ok(())
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
