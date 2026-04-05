use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
// Add to Cargo.toml:
// reqwest = { version = "0.11", features = ["blocking"] }

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

fn print_github_compare_links(
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
    verbose: bool,
) {
    if !verbose {
        return;
    }
    println!("\nGitHub compare links for updated crates:");
    for pkg in updated {
        if let Some(orig_pkg) = original.iter().find(|p| p.name == pkg.name) {
            if orig_pkg.version != pkg.version {
                if let Some(repo) = &pkg.repository {
                    if repo.contains("github.com") {
                        // Try to construct a compare link
                        let from = orig_pkg.version.to_string();
                        let to = pkg.version.to_string();
                        let repo_url = repo.trim_end_matches(".git");
                        println!("- {}: {}/compare/v{}...v{}", pkg.name, repo_url, from, to);
                    }
                }
            }
        }
    }
}

fn print_changelog_links(
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
    verbose: bool,
) {
    if !verbose {
        return;
    }
    println!("\nGuessed changelog links for updated crates:");
    for pkg in updated {
        if let Some(orig_pkg) = original.iter().find(|p| p.name == pkg.name) {
            if orig_pkg.version != pkg.version {
                if let Some(repo) = &pkg.repository {
                    if repo.contains("github.com") {
                        // Remove .git suffix and trailing slashes
                        let mut repo_url = repo
                            .trim_end_matches(".git")
                            .trim_end_matches('/')
                            .to_string();
                        // If repo_url contains /tree/ or /blob/, strip everything after that
                        if let Some(idx) = repo_url.find("/tree/") {
                            repo_url.truncate(idx);
                        } else if let Some(idx) = repo_url.find("/blob/") {
                            repo_url.truncate(idx);
                        }
                        // Only append /blob/master/CHANGELOG.md if not already in a subdir
                        let changelog_url = format!("{}/blob/master/CHANGELOG.md", repo_url);
                        println!("- {}: {}", pkg.name, changelog_url);
                    } else {
                        println!("- {}: <no GitHub repository>", pkg.name);
                    }
                } else {
                    println!("- {}: <no repository specified>", pkg.name);
                }
            }
        }
    }
}

fn print_changelog_entries(
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
    verbose: bool,
) {
    if !verbose {
        return;
    }
    println!("\nExtracted changelog entries for updated crates:");
    for pkg in updated {
        if let Some(orig_pkg) = original.iter().find(|p| p.name == pkg.name) {
            if orig_pkg.version != pkg.version {
                if let Some(repo) = &pkg.repository {
                    if repo.contains("github.com") {
                        let mut repo_url = repo
                            .trim_end_matches(".git")
                            .trim_end_matches('/')
                            .to_string();
                        if let Some(idx) = repo_url.find("/tree/") {
                            repo_url.truncate(idx);
                        } else if let Some(idx) = repo_url.find("/blob/") {
                            repo_url.truncate(idx);
                        }
                        // Try master and main branches
                        let branches = ["master", "main"];
                        let mut found = false;
                        for branch in &branches {
                            let raw_url = repo_url.replace(
                                "https://github.com/",
                                "https://raw.githubusercontent.com/",
                            ) + &format!("/{}/CHANGELOG.md", branch);
                            if let Ok(resp) = reqwest::blocking::get(&raw_url) {
                                if resp.status().is_success() {
                                    if let Ok(text) = resp.text() {
                                        println!("- {}:", pkg.name);
                                        // Try to find the section for the new version
                                        let version_header = format!("{}", pkg.version);
                                        let mut lines = text.lines();
                                        let mut printing = false;
                                        let mut count = 0;
                                        while let Some(line) = lines.next() {
                                            if line.contains(&version_header) {
                                                printing = true;
                                            } else if printing && line.starts_with('#') {
                                                break;
                                            }
                                            if printing {
                                                println!("    {}", line);
                                                count += 1;
                                                if count >= 10 {
                                                    break;
                                                }
                                            }
                                        }
                                        if !printing {
                                            // Print the first 10 lines as fallback
                                            for line in text.lines().take(10) {
                                                println!("    {}", line);
                                            }
                                        }
                                        found = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !found {
                            println!("- {}: <could not fetch or parse changelog>", pkg.name);
                        }
                    } else {
                        println!("- {}: <no GitHub repository>", pkg.name);
                    }
                } else {
                    println!("- {}: <no repository specified>", pkg.name);
                }
            }
        }
    }
}

fn print_changelog_entries_placeholder(verbose: bool) {
    if !verbose {
        return;
    }
    println!("\n[TODO] Extracted changelog entries for updated crates:");
    println!(
        "(This feature is not yet implemented. In the future, this will show the actual changelog entries for each updated crate.)"
    );
}

fn print_single_crate_update(
    crate_name: &str,
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
    verbose: bool,
) {
    let orig_pkg = original.iter().find(|p| p.name == crate_name);
    let updated_pkg = updated.iter().find(|p| p.name == crate_name);
    match (orig_pkg, updated_pkg) {
        (Some(orig), Some(updated)) => {
            if orig.version != updated.version {
                println!("{}: {} → {}", crate_name, orig.version, updated.version);
                // Always print changelog diff for this crate, regardless of verbose
                print_changelog_diff_for_crate(orig, updated, true);
            } else {
                println!("{}: no version change ({}).", crate_name, orig.version);
            }
        }
        (None, Some(updated)) => {
            println!("{}: newly added at version {}", crate_name, updated.version);
            print_changelog_diff_for_crate(updated, updated, true);
        }
        (Some(orig), None) => {
            println!("{}: removed (was at version {})", crate_name, orig.version);
        }
        (None, None) => {
            println!("{}: not found in either lockfile", crate_name);
        }
    }
}

fn print_changelog_diff_for_crate(
    orig: &cargo_metadata::Package,
    updated: &cargo_metadata::Package,
    verbose: bool,
) {
    if !verbose {
        return;
    }
    if let Some(repo) = &updated.repository {
        if repo.contains("github.com") {
            let mut repo_url = repo
                .trim_end_matches(".git")
                .trim_end_matches('/')
                .to_string();
            if let Some(idx) = repo_url.find("/tree/") {
                repo_url.truncate(idx);
            } else if let Some(idx) = repo_url.find("/blob/") {
                repo_url.truncate(idx);
            }
            let branches = ["master", "main"];
            let mut found = false;
            for branch in &branches {
                let raw_url = repo_url
                    .replace("https://github.com/", "https://raw.githubusercontent.com/")
                    + &format!("/{}/CHANGELOG.md", branch);
                if let Ok(resp) = reqwest::blocking::get(&raw_url) {
                    if resp.status().is_success() {
                        if let Ok(text) = resp.text() {
                            println!("Changelog diff for {}:", updated.name);
                            let from = orig.version.to_string();
                            let to = updated.version.to_string();
                            let mut lines = text.lines();
                            let mut printing = false;
                            let mut count = 0;
                            while let Some(line) = lines.next() {
                                if line.contains(&to) {
                                    printing = true;
                                } else if printing && line.starts_with('#') && !line.contains(&to) {
                                    break;
                                }
                                if printing {
                                    println!("    {}", line);
                                    count += 1;
                                    if count >= 20 {
                                        break;
                                    }
                                }
                            }
                            if !printing {
                                println!(
                                    "    (Could not find changelog section for version {})",
                                    to
                                );
                            }
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found {
                println!("    <could not fetch or parse changelog>");
            }
        } else {
            println!("    <no GitHub repository>");
        }
    } else {
        println!("    <no repository specified>");
    }
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let verbose = args.iter().any(|arg| arg == "-v" || arg == "--verbose");
    // Only treat a non-flag argument as a crate name if it is not the subcommand name
    let crate_arg = args
        .iter()
        .find(|a| !a.starts_with('-') && a != &"whats-new")
        .cloned();
    // Load metadata for the current workspace
    let metadata = MetadataCommand::new().exec()?;
    println!("Workspace root: {}", metadata.workspace_root);
    // Step 1: Create temp workspace and copy files
    let temp_dir = setup_temp_workspace(metadata.workspace_root.as_str(), verbose)?;
    // Step 2: Run cargo update in temp workspace
    run_cargo_update(temp_dir.path(), verbose)?;
    // Step 3: Load updated dependency graph from temp workspace
    let updated_metadata = load_metadata_from_path(temp_dir.path(), verbose)?;
    if let Some(crate_name) = crate_arg {
        print_single_crate_update(
            &crate_name,
            &metadata.packages,
            &updated_metadata.packages,
            verbose,
        );
        return Ok(());
    }
    // Step 4: Diff before/after versions
    diff_package_versions(&metadata.packages, &updated_metadata.packages, verbose);
    // Step 5: Report which crates were updated or added
    report_updated_crates(&metadata.packages, &updated_metadata.packages, verbose);
    // Step 6: Print repository URLs for all updated packages
    print_crate_repositories(&updated_metadata.packages, verbose);
    // Step 7: Print GitHub compare links for updated packages
    print_github_compare_links(&metadata.packages, &updated_metadata.packages, verbose);
    // Step 8: Print guessed changelog links for updated packages
    print_changelog_links(&metadata.packages, &updated_metadata.packages, verbose);
    // Step 9: Extract and print changelog entries for updated packages
    print_changelog_entries(&metadata.packages, &updated_metadata.packages, verbose);
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
