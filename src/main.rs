use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn setup_temp_workspace(workspace_root: &str, verbose: bool) -> Result<tempfile::TempDir> {
    use std::fs;
    use std::path::Path;
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

fn run_cargo_update(temp_path: &std::path::Path, verbose: bool) -> Result<()> {
    use std::process::{Command, Stdio};
    if verbose {
        println!("\n[DEBUG] Running 'cargo update' in temp workspace...");
    }
    let mut cmd = Command::new("cargo");
    cmd.arg("update").current_dir(temp_path);
    if !verbose {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let status = cmd.status()?;
    if status.success() {
        if verbose {
            println!("[DEBUG] 'cargo update' completed successfully.");
        }
        Ok(())
    } else {
        anyhow::bail!("'cargo update' failed in temp workspace");
    }
}

fn load_metadata_from_path(
    path: &std::path::Path,
    verbose: bool,
) -> Result<cargo_metadata::Metadata> {
    let mut cmd = MetadataCommand::new();
    cmd.current_dir(path);
    let metadata = cmd.exec()?;
    if verbose {
        println!("[DEBUG] Loaded updated dependency graph from temp workspace.");
    }
    Ok(metadata)
}

fn diff_package_versions(
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
    verbose: bool,
) {
    if !verbose {
        return;
    }
    // Map (name, source) -> version for both sets
    let mut orig_map: HashMap<(&str, Option<String>), String> = HashMap::new();
    for pkg in original {
        let key = (
            pkg.name.as_str(),
            pkg.source.as_ref().map(|s| s.to_string()),
        );
        let version = pkg.version.to_string();
        orig_map.insert(key, version);
    }
    let mut updated_map: HashMap<(&str, Option<String>), String> = HashMap::new();
    for pkg in updated {
        let key = (
            pkg.name.as_str(),
            pkg.source.as_ref().map(|s| s.to_string()),
        );
        let version = pkg.version.to_string();
        updated_map.insert(key, version);
    }
    println!("\nDependency changes (old → new):");
    for (key, old_ver) in &orig_map {
        if let Some(new_ver) = updated_map.get(key) {
            if old_ver != new_ver {
                println!("- {} ({} → {})", key.0, old_ver, new_ver);
            }
        }
    }
}

fn report_updated_crates(
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
    verbose: bool,
) {
    if !verbose {
        return;
    }
    use std::collections::HashSet;
    let orig_set: HashSet<_> = original.iter().map(|p| p.name.as_str()).collect();
    let updated_set: HashSet<_> = updated.iter().map(|p| p.name.as_str()).collect();
    let updated_crates: Vec<_> = updated_set.difference(&orig_set).collect();
    if !updated_crates.is_empty() {
        println!("\nNew crates added after update:");
        for &name in &updated_crates {
            println!("- {}", name);
        }
    }
    let mut changed = Vec::new();
    for pkg in updated {
        if let Some(orig_pkg) = original.iter().find(|p| p.name == pkg.name) {
            if orig_pkg.version != pkg.version {
                changed.push((&pkg.name, &orig_pkg.version, &pkg.version));
            }
        }
    }
    if !changed.is_empty() {
        println!("\nCrates updated:");
        for (name, old, new) in changed {
            println!("- {} ({} → {})", name, old, new);
        }
    }
}

fn print_crate_repositories(updated: &[cargo_metadata::Package], verbose: bool) {
    if !verbose {
        return;
    }
    println!("\nCrate repositories (after update):");
    for pkg in updated {
        if let Some(repo) = &pkg.repository {
            println!("- {}: {}", pkg.name, repo);
        } else {
            println!("- {}: <no repository specified>", pkg.name);
        }
    }
}

fn main() -> Result<()> {
    let verbose = env::args().any(|arg| arg == "-v" || arg == "--verbose");
    // Load metadata for the current workspace
    let metadata = MetadataCommand::new().exec()?;

    println!("Workspace root: {}", metadata.workspace_root);
    // Step 1: Create temp workspace and copy files
    let temp_dir = setup_temp_workspace(metadata.workspace_root.as_str(), verbose)?;
    // Step 2: Run cargo update in temp workspace
    run_cargo_update(temp_dir.path(), verbose)?;
    // Step 3: Load updated dependency graph from temp workspace
    let updated_metadata = load_metadata_from_path(temp_dir.path(), verbose)?;

    if verbose {
        println!("\nPackages (original):");
        for pkg in &metadata.packages {
            println!(
                "- {} {} ({})",
                pkg.name,
                pkg.version,
                pkg.manifest_path.as_str()
            );
        }
        println!("\nPackages (after update):");
        for pkg in &updated_metadata.packages {
            println!(
                "- {} {} ({})",
                pkg.name,
                pkg.version,
                pkg.manifest_path.as_str()
            );
        }
    }
    // Step 4: Diff before/after versions
    diff_package_versions(&metadata.packages, &updated_metadata.packages, verbose);
    // Step 5: Report which crates were updated or added
    report_updated_crates(&metadata.packages, &updated_metadata.packages, verbose);
    // Step 6: Print repository URLs for all updated packages
    print_crate_repositories(&updated_metadata.packages, verbose);
    // Minimal output: just show summary of updated crates
    if !verbose {
        let mut changed = Vec::new();
        for pkg in &updated_metadata.packages {
            if let Some(orig_pkg) = metadata.packages.iter().find(|p| p.name == pkg.name) {
                if orig_pkg.version != pkg.version {
                    changed.push((&pkg.name, &orig_pkg.version, &pkg.version));
                }
            }
        }
        if changed.is_empty() {
            println!("All dependencies are up to date.");
        } else {
            println!("Updated dependencies:");
            for (name, old, new) in changed {
                println!("- {} ({} → {})", name, old, new);
            }
        }
    }
    Ok(())
}
