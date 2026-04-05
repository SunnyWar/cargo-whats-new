use anyhow::Result;
use cargo_metadata::MetadataCommand;

fn main() -> Result<()> {
    // Load metadata for the current workspace
    let metadata = MetadataCommand::new().exec()?;

    println!("Workspace root: {}", metadata.workspace_root);

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
