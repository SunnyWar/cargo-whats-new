mod ops;
mod util;

use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::env;

use ops::{
    diff_package_versions, load_metadata_from_path, print_changelog_entries, print_changelog_links,
    print_crate_repositories, print_github_compare_links, print_minimal_updated_crates,
    print_single_crate_update, report_updated_crates, run_cargo_update,
};
use util::setup_temp_workspace;

fn main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let verbose = args.iter().any(|arg| arg == "-v" || arg == "--verbose");
    let crate_arg = args
        .iter()
        .find(|a| !a.starts_with('-') && a != &"whats-new")
        .cloned();
    let metadata = MetadataCommand::new().exec()?;
    println!("Workspace root: {}", metadata.workspace_root);
    let temp_dir = setup_temp_workspace(metadata.workspace_root.as_str(), verbose)?;
    run_cargo_update(temp_dir.path(), verbose)?;
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
    diff_package_versions(&metadata.packages, &updated_metadata.packages, verbose);
    report_updated_crates(&metadata.packages, &updated_metadata.packages, verbose);
    print_crate_repositories(&updated_metadata.packages, verbose);
    print_github_compare_links(&metadata.packages, &updated_metadata.packages, verbose);
    print_changelog_links(&metadata.packages, &updated_metadata.packages, verbose);
    print_changelog_entries(&metadata.packages, &updated_metadata.packages, verbose);
    if !verbose {
        print_minimal_updated_crates(&metadata.packages, &updated_metadata.packages);
    }
    Ok(())
}
