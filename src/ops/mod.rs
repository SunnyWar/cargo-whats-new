use crate::github_api::fetch_release_notes_from_github_api;
use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::collections::HashMap;
use std::fs;

pub fn run_cargo_update(temp_path: &std::path::Path, verbose: bool) -> Result<()> {
    use std::process::{Command, Stdio};
    if verbose {
        #[cfg(debug_assertions)]
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
            #[cfg(debug_assertions)]
            println!("[DEBUG] 'cargo update' completed successfully.");
        }
        Ok(())
    } else {
        anyhow::bail!("'cargo update' failed in temp workspace");
    }
}

pub fn load_metadata_from_path(
    path: &std::path::Path,
    verbose: bool,
) -> Result<cargo_metadata::Metadata> {
    let mut cmd = MetadataCommand::new();
    cmd.current_dir(path);
    let metadata = cmd.exec()?;
    if verbose {
        #[cfg(debug_assertions)]
        println!("[DEBUG] Loaded updated dependency graph from temp workspace.");
    }
    Ok(metadata)
}

pub fn diff_package_versions(
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
    verbose: bool,
) {
    if !verbose {
        return;
    }
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

pub fn report_updated_crates(
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

pub fn print_crate_repositories(updated: &[cargo_metadata::Package], verbose: bool) {
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

pub fn print_github_compare_links(
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

pub fn print_changelog_links(
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
                        let mut repo_url = repo
                            .trim_end_matches(".git")
                            .trim_end_matches('/')
                            .to_string();
                        if let Some(idx) = repo_url.find("/tree/") {
                            repo_url.truncate(idx);
                        } else if let Some(idx) = repo_url.find("/blob/") {
                            repo_url.truncate(idx);
                        }
                        let branches = ["main", "master"];
                        let mut found = false;
                        for branch in &branches {
                            let changelog_url =
                                format!("{}/blob/{}/CHANGELOG.md", repo_url, branch);
                            let raw_url = repo_url.replace(
                                "https://github.com/",
                                "https://raw.githubusercontent.com/",
                            ) + &format!("/{}/CHANGELOG.md", branch);
                            if let Ok(resp) = reqwest::blocking::get(&raw_url) {
                                if resp.status().is_success() {
                                    println!("- {}: {}", pkg.name, changelog_url);
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            println!(
                                "- {}: <could not find CHANGELOG.md on main or master>",
                                pkg.name
                            );
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

pub fn print_changelog_entries(
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
                            // Try GitHub releases page as fallback
                            if let Some(notes) =
                                try_fetch_github_release_notes(&repo_url, &pkg.version.to_string())
                            {
                                println!("- {} (from GitHub releases):\n{}", pkg.name, notes);
                            } else {
                                println!("- {}: <could not fetch or parse changelog>", pkg.name);
                            }
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

pub fn print_single_crate_update(
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
                print_changelog_diff_for_crate_verbose(orig, updated, verbose, crate_name);
            } else {
                println!("{}: no version change ({}).", crate_name, orig.version);
            }
        }
        (None, Some(updated)) => {
            println!("{}: newly added at version {}", crate_name, updated.version);
            print_changelog_diff_for_crate_verbose(updated, updated, verbose, crate_name);
        }
        (Some(orig), None) => {
            println!("{}: removed (was at version {})", crate_name, orig.version);
        }
        (None, None) => {
            println!("{}: not found in either lockfile", crate_name);
        }
    }
}

fn print_changelog_diff_for_crate_verbose(
    _orig: &cargo_metadata::Package,
    updated: &cargo_metadata::Package,
    verbose: bool,
    crate_name: &str,
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
                            let to = updated.version.to_string();
                            let re = regex::Regex::new(&format!(
                                r"^#+\s*\[?v?{}\]?\s*$",
                                regex::escape(&to)
                            ))
                            .unwrap();
                            let mut lines = text.lines().peekable();
                            let mut printing = false;
                            let mut count = 0;
                            while let Some(line) = lines.next() {
                                if !printing {
                                    if re.is_match(line) {
                                        printing = true;
                                        println!("    {}", line);
                                        count += 1;
                                    }
                                } else {
                                    if let Some(next_line) = lines.peek() {
                                        if next_line.trim_start().starts_with('#') {
                                            let next_header_re =
                                                regex::Regex::new(r"^#+\s*\[?v?[0-9]+\.").unwrap();
                                            if next_header_re.is_match(next_line.trim_start()) {
                                                break;
                                            }
                                        }
                                    }
                                    println!("    {}", line);
                                    count += 1;
                                    if count >= 100 {
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
                // Try GitHub releases page as fallback
                if let Some(notes) =
                    try_fetch_github_release_notes(&repo_url, &updated.version.to_string())
                {
                    println!("- {} (from GitHub releases):\n{}", crate_name, notes);
                } else {
                    println!("- {}: <could not fetch or parse changelog>", crate_name);
                }
            }
        } else {
            println!("    <no GitHub repository>");
        }
    } else {
        println!("    <no repository specified>");
    }
}

fn fetch_github_release_tag_html(owner: &str, repo: &str, version: &str) -> Option<String> {
    let tag_variants = [version.to_string(), format!("v{}", version)];
    for tag in &tag_variants {
        let tag_url = format!("https://github.com/{}/{}/releases/tag/{}", owner, repo, tag);
        if let Ok(resp) = reqwest::blocking::get(&tag_url) {
            if resp.status().is_success() {
                if let Ok(text) = resp.text() {
                    let _ = fs::write("release_debug.html", &text);
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[DEBUG] Saved fetched HTML to release_debug.html ({} bytes)",
                        text.len()
                    );
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[DEBUG] Fetched tag page: {} ({} bytes)",
                        tag_url,
                        text.len()
                    );
                    return Some(text);
                }
            } else {
                eprintln!(
                    "[DEBUG] Failed to fetch tag page: {} (status: {})",
                    tag_url,
                    resp.status()
                );
            }
        } else {
            #[cfg(debug_assertions)]
            eprintln!("[DEBUG] Error fetching tag page: {}", tag_url);
        }
    }
    None
}

fn extract_markdown_body_blocks(html: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut offset = 0;
    while let Some(start) = html[offset..].find("<div class=\"markdown-body") {
        let abs_start = offset + start;
        let after = &html[abs_start..];
        if let Some(end) = after.find("</div>") {
            let block = &after[..end + 6];
            blocks.push(block.to_string());
            offset = abs_start + end + 6;
        } else {
            break;
        }
    }
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG] Found {} markdown-body blocks", blocks.len());
    blocks
}

fn html_to_plain_text(html: &str) -> String {
    let plain = html.replace("<br>", "\n");
    let plain = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&plain, "");
    let plain = html_escape::decode_html_entities(&plain);
    plain.trim().to_string()
}

fn extract_first_meaningful_block(blocks: &[String], version: &str) -> Option<String> {
    // Prefer a block containing the version string, else the largest block
    let mut best: Option<&String> = None;
    for block in blocks {
        let plain = html_to_plain_text(block);
        if plain.contains(version) && plain.len() > 20 {
            #[cfg(debug_assertions)]
            eprintln!(
                "[DEBUG] Selected markdown-body block with version ({} chars)",
                plain.len()
            );
            return Some(plain);
        }
        if best.is_none() || plain.len() > html_to_plain_text(best.unwrap()).len() {
            best = Some(block);
        }
    }
    best.map(|b| {
        let plain = html_to_plain_text(b);
        #[cfg(debug_assertions)]
        eprintln!(
            "[DEBUG] Selected largest markdown-body block ({} chars)",
            plain.len()
        );
        plain
    })
}

fn extract_fallback_article_content(html: &str) -> Option<String> {
    if let Some(start) = html.find("<article") {
        let after = &html[start..];
        if let Some(end) = after.find("</article>") {
            let block = &after[..end + 10];
            let plain = html_to_plain_text(block);
            let lines: Vec<_> = plain.lines().filter(|l| !l.trim().is_empty()).collect();
            if !lines.is_empty() {
                let result = lines.join("\n");
                #[cfg(debug_assertions)]
                eprintln!("[DEBUG] Fallback article content ({} chars)", result.len());
                return Some(result);
            }
        }
    }
    None
}

fn try_fetch_github_release_notes(repo_url: &str, version: &str) -> Option<String> {
    if !repo_url.contains("github.com") {
        return None;
    }
    let parts: Vec<&str> = repo_url.trim_end_matches(".git").split('/').collect();
    if parts.len() < 5 {
        return None;
    }
    let owner = parts[3];
    let repo = parts[4];
    // Try to fetch release notes from the tag page (HTML scraping)
    let html = fetch_github_release_tag_html(owner, repo, version)?;
    let blocks = extract_markdown_body_blocks(&html);
    if let Some(plain) = extract_first_meaningful_block(&blocks, version) {
        return Some(plain);
    }
    if let Some(article) = extract_fallback_article_content(&html) {
        return Some(article);
    }
    // If all else fails, try the GitHub API (if token is set)
    fetch_release_notes_from_github_api(owner, repo, version)
}

pub fn print_minimal_updated_crates(
    original: &[cargo_metadata::Package],
    updated: &[cargo_metadata::Package],
) {
    let mut changed = Vec::new();
    for pkg in updated {
        if let Some(orig_pkg) = original.iter().find(|p| p.name == pkg.name) {
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
