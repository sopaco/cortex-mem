use anyhow::Result;
use colored::Colorize;
use std::path::Path;

/// List all available tenants
pub async fn list(data_dir: &str) -> Result<()> {
    println!("{} Listing all available tenants", "📋".bold());
    
    let tenants_dir = Path::new(data_dir).join("tenants");
    
    if !tenants_dir.exists() {
        println!("\n{} No tenants directory found at {}", "ℹ".yellow().bold(), tenants_dir.display());
        return Ok(());
    }
    
    let mut tenants = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&tenants_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    // Skip hidden directories
                    if !name.starts_with('.') {
                        tenants.push(name.to_string());
                    }
                }
            }
        }
    }
    
    if tenants.is_empty() {
        println!("\n{} No tenants found", "ℹ".yellow().bold());
        println!("\n  Data directory: {}", tenants_dir.display().to_string().dimmed());
        return Ok(());
    }
    
    // Sort tenants alphabetically
    tenants.sort();
    
    println!("\n{} Found {} tenant(s):", "✓".green().bold(), tenants.len());
    println!();
    
    for tenant in tenants {
        println!("• {}", tenant.bright_blue().bold());
    }
    
    println!("\n  {} Use --tenant <id> to specify which tenant to operate on", "💡".dimmed());
    println!("  {} Data directory: {}", "📁".dimmed(), tenants_dir.display().to_string().dimmed());
    
    Ok(())
}
