use anyhow::Result;
use colored::Colorize;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

/// Reindex: clean up no-URI vectors, then full sync
pub async fn reindex(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("{} Starting vector reindex...\n", "🔄".bold());

    // Step 1: delete stale (no-URI) vectors using gRPC
    let vector_store = operations.vector_store();
    let collection_name = vector_store.collection_name().to_string();
    
    match vector_store.delete_no_uri_points(&collection_name).await {
        Ok(n) => println!("  {} Removed {} stale vectors (no URI metadata)", "✅".green(), n),
        Err(e) => println!("  {} Failed to clean stale vectors: {} (continuing...)", "⚠️".yellow(), e),
    }

    // Step 2: full sync
    println!("\n{} Syncing all files to vector database...", "📦".bold());
    let stats = operations.index_all_files().await?;

    println!("\n{} Reindex complete!", "✅".bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Statistics:");
    println!("   • Files processed:       {}", stats.total_files);
    println!("   • Newly indexed:         {}", stats.indexed_files);
    println!("   • Skipped (up-to-date):  {}", stats.skipped_files);
    println!("   • Errors:                {}", stats.error_files);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if stats.error_files > 0 {
        println!("\n⚠️  Some files failed to index. Run with --verbose for details.");
    }

    Ok(())
}

/// Prune: delete vectors whose corresponding files no longer exist on disk
pub async fn prune(operations: Arc<MemoryOperations>, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("{} Scanning for dangling vectors (dry-run, no changes)...\n", "🔍".bold());
    } else {
        println!("{} Scanning for dangling vectors (files deleted from disk)...\n", "🧹".bold());
    }

    let vector_store = operations.vector_store();
    let collection_name = vector_store.collection_name().to_string();
    
    // Get all points with URI using gRPC scroll
    let points = vector_store
        .scroll_points_with_uri(&collection_name, 200)
        .await?;

    let mut total_checked = 0u64;
    let mut dangling_ids: Vec<String> = vec![];

    for (point_id, uri_opt) in &points {
        total_checked += 1;
        
        let uri = match uri_opt {
            Some(u) if !u.is_empty() => u,
            _ => continue,
        };

        // Check if the file still exists in cortex filesystem
        let exists = operations.exists(uri).await.unwrap_or(true); // assume exists on error
        if !exists {
            if dry_run {
                println!("  {} would delete: {}", "→".dimmed(), uri);
            }
            dangling_ids.push(point_id.clone());
        }
    }

    println!("\n  Checked {} vectors", total_checked);
    println!("  Dangling (file missing): {}", dangling_ids.len());

    if dangling_ids.is_empty() {
        println!("\n{} No dangling vectors found.", "✅".green());
        return Ok(());
    }

    if dry_run {
        println!(
            "\n{} Dry-run complete. Run without --dry-run to actually delete {} vectors.",
            "ℹ️".cyan(), dangling_ids.len()
        );
        return Ok(());
    }

    // Delete dangling points using gRPC
    vector_store.delete_points_by_ids(&collection_name, &dangling_ids).await?;

    println!("\n{} Pruned {} dangling vectors.", "✅".green(), dangling_ids.len());
    Ok(())
}

/// Show vector index status for the current tenant
pub async fn status(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("{} Vector index status\n", "📊".bold());

    // Count total tracked files
    let mut total_files = 0usize;
    for root in &["cortex://session", "cortex://user", "cortex://agent"] {
        if let Ok(files) = operations.list_files(root).await {
            total_files += files.len();
        }
    }
    println!("  Total tracked files: ~{}", total_files);

    match fetch_collection_stats(operations).await {
        Ok((total_pts, no_uri_pts)) => {
            println!("  Vectors in Qdrant:   {}", total_pts);
            if no_uri_pts > 0 {
                println!(
                    "  Missing URI (stale): {} {}",
                    no_uri_pts,
                    "(run `vector reindex` to fix)".yellow()
                );
            } else {
                println!("  Missing URI (stale): 0 ✅");
            }
        }
        Err(e) => {
            println!("  {} Could not reach Qdrant: {}", "⚠️".yellow(), e);
        }
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn fetch_collection_stats(operations: Arc<MemoryOperations>) -> Result<(u64, u64)> {
    let vector_store = operations.vector_store();
    let collection_name = vector_store.collection_name().to_string();
    
    // Get total points count
    let total_pts = vector_store.get_collection_points_count(&collection_name).await?;
    
    // Get no-URI points count
    let no_uri_pts = vector_store.count_no_uri_points(&collection_name).await?;

    Ok((total_pts, no_uri_pts))
}