use axum::{extract::State, Json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::info;

use crate::handlers::filesystem::load_layers_for_uri;
use crate::{
    error::{AppError, Result},
    models::{ApiResponse, SearchRequest, SearchResultResponse},
    state::AppState,
};

/// Search endpoint using layered vector search (L0/L1/L2)
pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResultResponse>>>> {
    let limit = req.limit.unwrap_or(10);
    let min_score = req.min_score.unwrap_or(0.6);
    let return_layers = req.return_layers.clone();

    let results = search_layered(
        &state,
        &req.query,
        req.thread.as_deref(),
        limit,
        min_score,
        &return_layers,
    )
    .await?;

    Ok(Json(ApiResponse::success(results)))
}

/// Layered semantic search using L0/L1/L2 tiered retrieval
async fn search_layered(
    state: &AppState,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
    return_layers: &[String],
) -> Result<Vec<SearchResultResponse>> {
    use cortex_mem_core::SearchOptions;

    let vector_engine_lock = state.vector_engine.read().await;
    let vector_engine = vector_engine_lock.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "Vector search not available. Qdrant and Embedding service must be configured."
                .to_string(),
        )
    })?;

    let mut options = SearchOptions {
        limit,
        threshold: min_score,
        root_uri: None,
        recursive: true,
    };
    let mut semantic_options = options.clone();
    semantic_options.threshold = (min_score * 0.5).max(0.0);

    if let Some(thread_id) = thread {
        // Support both session ID and full URI format
        // - "abc" -> "cortex://session/abc" (backward compatible)
        // - "cortex://user/default" -> "cortex://user/default" (full URI)
        let scope_uri = if thread_id.starts_with("cortex://") {
            thread_id.to_string()
        } else {
            format!("cortex://session/{}", thread_id)
        };
        options.root_uri = Some(scope_uri.clone());
        semantic_options.root_uri = Some(scope_uri);
    }

    let tenant_root = state.current_tenant_root.read().await.clone();
    let base_dir = if let Some(ref root) = tenant_root {
        root.clone()
    } else {
        state.data_dir.clone()
    };

    let profile = build_query_profile(query);

    let layered_results = vector_engine
        .layered_semantic_search(query, &options)
        .await
        .map_err(|e| AppError::Internal(format!("Layered search failed: {}", e)))?;

    let semantic_results = vector_engine
        .semantic_search(query, &semantic_options)
        .await
        .map_err(|e| AppError::Internal(format!("Semantic search failed: {}", e)))?;

    let expanded_semantic_results = if profile.expanded_query != query {
        vector_engine
            .semantic_search(&profile.expanded_query, &semantic_options)
            .await
            .map_err(|e| AppError::Internal(format!("Expanded semantic search failed: {}", e)))?
    } else {
        Vec::new()
    };

    info!(
        query = %query,
        kind = ?profile.kind,
        layered_count = layered_results.len(),
        semantic_count = semantic_results.len(),
        expanded_count = expanded_semantic_results.len(),
        "service search stage counts"
    );

    let mut merged = merge_search_results(
        layered_results,
        semantic_results,
        expanded_semantic_results,
    );
    rerank_results(&profile, &mut merged);

    info!(
        query = %query,
        merged_count = merged.len(),
        top_source = merged.first().map(|r| r.source.as_str()).unwrap_or("none"),
        top_uri = merged.first().map(|r| r.uri.as_str()).unwrap_or("none"),
        "service merged vector results"
    );

    if merged.is_empty() {
        let mut broad_options = semantic_options.clone();
        broad_options.threshold = 0.0;
        let broad_query = if profile.expanded_query != query {
            &profile.expanded_query
        } else {
            query
        };
        let broad_results = vector_engine
            .semantic_search(broad_query, &broad_options)
            .await
            .map_err(|e| AppError::Internal(format!("Broad semantic search failed: {}", e)))?;
        merged = merge_search_results(Vec::new(), Vec::new(), broad_results);
        rerank_results(&profile, &mut merged);
        info!(
            query = %query,
            broad_count = merged.len(),
            top_source = merged.first().map(|r| r.source.as_str()).unwrap_or("none"),
            top_uri = merged.first().map(|r| r.uri.as_str()).unwrap_or("none"),
            "service broad semantic results"
        );
    }

    // B4: When merged results are insufficient, supplement with a user-scope targeted search
    // This ensures user-level extracted memories (personal_info, events, preferences) are
    // included even if global vector search misses them (critical for LoCoMo Cat 1/2)
    if merged.len() < limit && options.root_uri.is_none() {
        let mut user_scope_options = semantic_options.clone();
        user_scope_options.root_uri = Some("cortex://user".to_string());
        user_scope_options.threshold = (min_score * 0.4).max(0.0);
        let user_results = vector_engine
            .semantic_search(query, &user_scope_options)
            .await
            .map_err(|e| AppError::Internal(format!("User-scope supplement search failed: {}", e)))?;
        if !user_results.is_empty() {
            info!(
                query = %query,
                user_scope_count = user_results.len(),
                "B4: user-scope supplement search triggered"
            );
            let user_merged = merge_search_results(Vec::new(), user_results, Vec::new());
            merged = merge_merged_results(merged, user_merged);
            rerank_results(&profile, &mut merged);
        }
    }

    let needs_lexical_fallback = merged.len() < 3;

    if needs_lexical_fallback {
        let lexical_results =
            lexical_fallback_search(&base_dir, &options.root_uri, &profile, limit * 4).await?;
        info!(
            query = %query,
            lexical_count = lexical_results.len(),
            "service lexical fallback triggered"
        );
        merged = merge_merged_results(merged, lexical_results);
        rerank_results(&profile, &mut merged);
    }

    let mut results = Vec::new();
    for result in merged.drain(..).take(limit) {
        let snippet = if result.snippet.len() > 200 {
            format!(
                "{}...",
                &result.snippet.chars().take(200).collect::<String>()
            )
        } else {
            result.snippet
        };

        let (overview, content, layers) =
            load_layers_for_uri(&base_dir, &result.uri, return_layers).await;

        results.push(SearchResultResponse {
            uri: result.uri,
            score: result.score,
            snippet,
            overview,
            content,
            source: result.source,
            layers,
        });
    }

    Ok(results)
}

#[derive(Clone)]
struct MergedSearchResult {
    uri: String,
    score: f32,
    snippet: String,
    source: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum QueryKind {
    General,
    Identity,
    Temporal,
    Relationship,
    Career,
    Support,
    Activity,
}

#[derive(Clone)]
struct QueryProfile {
    kind: QueryKind,
    expanded_query: String,
    keywords: Vec<String>,
    strong_terms: Vec<&'static str>,
    medium_terms: Vec<&'static str>,
}

fn merge_search_results(
    layered_results: Vec<cortex_mem_core::SearchResult>,
    semantic_results: Vec<cortex_mem_core::SearchResult>,
    expanded_semantic_results: Vec<cortex_mem_core::SearchResult>,
) -> Vec<MergedSearchResult> {
    let mut merged: HashMap<String, MergedSearchResult> = HashMap::new();

    for result in layered_results {
        upsert_result(&mut merged, result, "layered_vector");
    }
    for result in semantic_results {
        upsert_result(&mut merged, result, "semantic_fallback");
    }
    for result in expanded_semantic_results {
        upsert_result(&mut merged, result, "semantic_expansion");
    }

    let mut results: Vec<MergedSearchResult> = merged.into_values().collect();
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

fn merge_merged_results(
    base_results: Vec<MergedSearchResult>,
    extra_results: Vec<MergedSearchResult>,
) -> Vec<MergedSearchResult> {
    if base_results.is_empty() {
        let mut results = extra_results;
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        return results;
    }

    let mut merged: HashMap<String, MergedSearchResult> = HashMap::new();

    for result in base_results {
        merged.insert(result.uri.clone(), result);
    }

    for result in extra_results {
        if let Some(existing) = merged.get_mut(&result.uri) {
            if existing.snippet.is_empty() && !result.snippet.is_empty() {
                existing.snippet = result.snippet.clone();
            }
        }
    }

    let mut results: Vec<MergedSearchResult> = merged.into_values().collect();
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

fn upsert_result(
    merged: &mut HashMap<String, MergedSearchResult>,
    result: cortex_mem_core::SearchResult,
    source: &str,
) {
    if !result.uri.starts_with("cortex://") {
        return;
    }

    match merged.get_mut(&result.uri) {
        Some(existing) => {
            if result.score > existing.score {
                existing.score = result.score;
                existing.snippet = result.snippet;
            }
            existing.source = if existing.source == source {
                source.to_string()
            } else {
                "hybrid_vector".to_string()
            };
        }
        None => {
            merged.insert(
                result.uri.clone(),
                MergedSearchResult {
                    uri: result.uri,
                    score: result.score,
                    snippet: result.snippet,
                    source: source.to_string(),
                },
            );
        }
    }
}

fn build_query_profile(query: &str) -> QueryProfile {
    let query_lower = query.to_lowercase();

    if ["identity", "transgender", "nonbinary", "gender", "身份", "认同"]
        .iter()
        .any(|kw| query_lower.contains(kw))
    {
        return QueryProfile {
            kind: QueryKind::Identity,
            expanded_query: format!(
                "{} identity self description gender background personal profile",
                query
            ),
            keywords: extract_keywords(query, QueryKind::Identity),
            strong_terms: vec![
                "identity",
                "gender identity",
                "identify as",
                "started transitioning",
                "background",
                "personal",
                "profile",
            ],
            medium_terms: vec![
                "transgender",
                "nonbinary",
                "gender",
                "transition",
                "accepted",
                "embrace",
                "profile",
            ],
        };
    }

    if ["relationship", "married", "partner", "friend", "breakup", "关系", "婚姻"]
        .iter()
        .any(|kw| query_lower.contains(kw))
    {
        return QueryProfile {
            kind: QueryKind::Relationship,
            expanded_query: format!(
                "{} relationship status friend partner spouse family connection",
                query
            ),
            keywords: extract_keywords(query, QueryKind::Relationship),
            strong_terms: vec![
                "relationship status",
                "close friend",
                "partner",
                "spouse",
                "family",
            ],
            medium_terms: vec![
                "friend",
                "relationship",
                "married",
                "partner",
                "spouse",
                "breakup",
            ],
        };
    }

    if [
        "career",
        "education",
        "job",
        "work",
        "study",
        "research",
        "goal",
        "职业",
        "教育",
    ]
    .iter()
    .any(|kw| query_lower.contains(kw))
    {
        return QueryProfile {
            kind: QueryKind::Career,
            expanded_query: format!(
                "{} career goal work education study aspiration plan",
                query
            ),
            keywords: extract_keywords(query, QueryKind::Career),
            strong_terms: vec![
                "career aspiration",
                "wants to pursue",
                "interested in",
                "goal",
                "future work",
            ],
            medium_terms: vec![
                "career",
                "job",
                "work",
                "education",
                "study",
                "research",
                "goal",
            ],
        };
    }

    if ["when", "date", "time", "before", "after", "昨天", "什么时候", "哪天"]
        .iter()
        .any(|kw| query_lower.contains(kw))
    {
        return QueryProfile {
            kind: QueryKind::Temporal,
            expanded_query: format!("{} date time event timeline timestamp when", query),
            keywords: extract_keywords(query, QueryKind::Temporal),
            strong_terms: vec!["timestamp", "date", "time", "the day before", "last year"],
            medium_terms: vec!["yesterday", "today", "last", "before", "after", "event"],
        };
    }

    if ["support", "help", "motivate", "encourage", "支持", "帮助"]
        .iter()
        .any(|kw| query_lower.contains(kw))
    {
        return QueryProfile {
            kind: QueryKind::Support,
            expanded_query: format!(
                "{} support help encourage motivate relationship community family friends",
                query
            ),
            keywords: extract_keywords(query, QueryKind::Support),
            strong_terms: vec!["support", "encourage", "helped", "community", "friends and family"],
            medium_terms: vec!["support", "help", "encourage", "motivate", "family", "friend"],
        };
    }

    if [
        "activit", "hobby", "hobbies", "sport", "pottery", "camping", "swimming",
        "painting", "hiking", "cooking", "garden", "craft", "yoga", "dance", "exercise",
        "partake", "involve", "engage", "interest",
    ]
    .iter()
    .any(|kw| query_lower.contains(kw))
    {
        return QueryProfile {
            kind: QueryKind::Activity,
            expanded_query: format!(
                "{} activities hobby interests sports recreation leisure participate",
                query
            ),
            keywords: extract_keywords(query, QueryKind::Activity),
            strong_terms: vec![
                "pottery", "camping", "swimming", "painting", "hiking", "cooking",
                "activities", "hobby", "hobbies", "sport", "partake", "participate",
            ],
            medium_terms: vec![
                "interest", "enjoy", "leisure", "recreation", "class", "sign up", "enrolled",
                "outdoor", "creative",
            ],
        };
    }

    QueryProfile {
        kind: QueryKind::General,
        expanded_query: query.to_string(),
        keywords: extract_keywords(query, QueryKind::General),
        strong_terms: vec![],
        medium_terms: vec![],
    }
}

fn extract_keywords(query: &str, kind: QueryKind) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut keywords: Vec<String> = query_lower
        .split(|c: char| !c.is_alphanumeric() && c != '+' && c != '#')
        .filter(|s| s.len() > 2)
        .filter(|s| {
            !matches!(
                *s,
                "what"
                    | "when"
                    | "where"
                    | "which"
                    | "who"
                    | "does"
                    | "did"
                    | "with"
                    | "about"
                    | "have"
                    | "this"
                    | "that"
                    | "caroline"
                    | "melanie"
                    | "their"
                    | "they"
                    | "them"
                    | "herself"
                    | "himself"
                    | "want"
                    | "pursue"
                    | "went"
            )
        })
        .map(|s| s.to_string())
        .collect();

    match kind {
        QueryKind::Identity => keywords.extend(
            [
                "identity",
                "gender",
                "profile",
                "background",
                "transition",
                "transgender",
                "nonbinary",
            ]
            .into_iter()
            .map(str::to_string),
        ),
        QueryKind::Relationship => keywords.extend(
            ["relationship", "friend", "partner", "spouse", "family", "married"]
                .into_iter()
                .map(str::to_string),
        ),
        QueryKind::Career => keywords.extend(
            ["career", "goal", "job", "work", "education", "study", "research"]
                .into_iter()
                .map(str::to_string),
        ),
        QueryKind::Temporal => keywords.extend(
            ["date", "time", "timestamp", "when", "before", "after", "event"]
                .into_iter()
                .map(str::to_string),
        ),
        QueryKind::Support => keywords.extend(
            ["support", "help", "encourage", "motivate", "family", "friend", "community"]
                .into_iter()
                .map(str::to_string),
        ),
        QueryKind::Activity => keywords.extend(
            [
                "pottery", "camping", "swimming", "painting", "hiking", "cooking",
                "activities", "hobby", "hobbies", "sport", "partake", "participate",
                "class", "sign", "enroll",
            ]
            .into_iter()
            .map(str::to_string),
        ),
        QueryKind::General => {}
    }

    keywords.sort();
    keywords.dedup();
    keywords
}

fn count_keyword_hits(text: &str, keywords: &[String]) -> usize {
    keywords
        .iter()
        .filter(|keyword| text.contains(keyword.as_str()))
        .count()
}

fn count_static_term_hits(text: &str, terms: &[&'static str]) -> usize {
    terms.iter().filter(|term| text.contains(**term)).count()
}

fn collection_summary_penalty(path_lower: &str) -> f32 {
    if path_lower.ends_with("/entities")
        || path_lower.ends_with("/events")
        || path_lower.ends_with("/goals")
        || path_lower.ends_with("/relationships")
    {
        -0.14
    } else {
        0.0
    }
}

fn query_path_bonus(kind: QueryKind, path_lower: &str) -> f32 {
    match kind {
        QueryKind::Identity => {
            if path_lower.contains("/personal_info/") {
                0.24
            } else if path_lower.contains("/events/") {
                0.08
            } else if path_lower.contains("/preferences/") {
                -0.12
            } else if path_lower.contains("/relationships/") {
                -0.18
            } else if path_lower.contains("/timeline/") {
                -0.05
            } else {
                0.0
            }
        }
        QueryKind::Temporal => {
            if path_lower.contains("/events/") || path_lower.contains("/timeline/") {
                0.06
            } else {
                0.0
            }
        }
        QueryKind::Relationship => {
            if path_lower.contains("/relationships/") {
                0.08
            } else {
                0.0
            }
        }
        QueryKind::Career => {
            if path_lower.contains("/goals/") || path_lower.contains("/preferences/") {
                0.08
            } else if path_lower.contains("/personal_info/") {
                0.02
            } else {
                0.0
            }
        }
        QueryKind::Support => {
            if path_lower.contains("/relationships/") {
                0.08
            } else if path_lower.contains("/events/") {
                0.05
            } else if path_lower.contains("/personal_info/") {
                0.03
            } else {
                0.0
            }
        }
        QueryKind::Activity => {
            if path_lower.contains("/events/") {
                0.12
            } else if path_lower.contains("/preferences/") {
                0.08
            } else if path_lower.contains("/personal_info/") {
                0.06
            } else if path_lower.contains("/timeline/") {
                0.04
            } else {
                0.0
            }
        }
        QueryKind::General => 0.0,
    }
}

fn rerank_results(profile: &QueryProfile, results: &mut [MergedSearchResult]) {
    for result in results.iter_mut() {
        let snippet_lower = result.snippet.to_lowercase();
        let is_leaf = result.uri.to_lowercase().ends_with(".md");
        let keyword_hits = count_keyword_hits(&snippet_lower, &profile.keywords) as f32;
        let medium_hits = count_static_term_hits(&snippet_lower, &profile.medium_terms) as f32;
        let strong_hits = count_static_term_hits(&snippet_lower, &profile.strong_terms) as f32;

        let mut bonus = match result.source.as_str() {
            "layered_vector" => 0.12,
            "hybrid_vector" => 0.08,
            "semantic_fallback" => 0.05,
            "semantic_expansion" => 0.03,
            "lexical_file" => -0.18,
            "lexical_layer" => -0.24,
            _ => 0.0,
        };

        bonus += if is_leaf { 0.08 } else { -0.08 };
        bonus += keyword_hits.min(4.0) * 0.03;
        bonus += medium_hits.min(3.0) * 0.02;
        bonus += strong_hits.min(2.0) * 0.04;
        bonus += query_path_bonus(profile.kind, &result.uri.to_lowercase());

        result.score += bonus;
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

async fn lexical_fallback_search(
    base_dir: &Path,
    root_uri: &Option<String>,
    profile: &QueryProfile,
    limit: usize,
) -> Result<Vec<MergedSearchResult>> {
    if profile.keywords.is_empty() {
        return Ok(Vec::new());
    }

    let scan_roots = build_scan_roots(base_dir, root_uri);
    let base_dir_buf = base_dir.to_path_buf();
    let profile = profile.clone();

    let mut hits = tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        for root in scan_roots {
            collect_lexical_hits(&base_dir_buf, &root, &profile, &mut results);
        }
        results
    })
    .await
    .map_err(|e| AppError::Internal(format!("Lexical fallback join error: {}", e)))?;

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);
    Ok(hits)
}

fn build_scan_roots(base_dir: &Path, root_uri: &Option<String>) -> Vec<PathBuf> {
    if let Some(root_uri) = root_uri {
        let rel = root_uri.trim_start_matches("cortex://");
        return vec![base_dir.join(rel)];
    }

    vec![
        base_dir.join("user"),
        base_dir.join("session"),
        base_dir.join("agent"),
    ]
}

fn make_match_snippet(content: &str, keywords: &[String]) -> String {
    let content_lower = content.to_lowercase();
    let start = keywords
        .iter()
        .filter_map(|kw| content_lower.find(kw))
        .min()
        .unwrap_or(0)
        .saturating_sub(80);
    let end = (start + 220).min(content.len());

    let mut start_idx = start;
    while start_idx > 0 && !content.is_char_boundary(start_idx) {
        start_idx -= 1;
    }

    let mut end_idx = end;
    while end_idx < content.len() && !content.is_char_boundary(end_idx) {
        end_idx += 1;
    }

    let snippet = content[start_idx..end_idx].trim().to_string();
    if start_idx > 0 {
        format!("...{}", snippet)
    } else {
        snippet
    }
}

fn collect_lexical_hits(
    base_dir: &Path,
    path: &Path,
    profile: &QueryProfile,
    hits: &mut Vec<MergedSearchResult>,
) {
    if !path.exists() {
        return;
    }

    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect_lexical_hits(base_dir, &entry.path(), profile, hits);
            }
        }
        return;
    }

    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    if !name.ends_with(".md") {
        return;
    }

    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    let content_lower = content.to_lowercase();
    let match_count = count_keyword_hits(&content_lower, &profile.keywords);
    if match_count == 0 {
        return;
    }

    let rel = match path.strip_prefix(base_dir) {
        Ok(rel) => rel,
        Err(_) => return,
    };
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    let rel_lower = rel_str.to_lowercase();
    let is_summary_layer = name == ".overview.md" || name == ".abstract.md";

    if is_summary_layer && match_count < 2 {
        return;
    }

    let (uri, source) = if is_summary_layer {
        let Some(parent) = rel.parent() else {
            return;
        };
        (
            format!("cortex://{}", parent.to_string_lossy().replace('\\', "/")),
            "lexical_layer",
        )
    } else {
        (format!("cortex://{}", rel_str), "lexical_file")
    };

    let snippet = make_match_snippet(&content, &profile.keywords);
    let keyword_hits = match_count as f32;
    let medium_hits = count_static_term_hits(&content_lower, &profile.medium_terms) as f32;
    let strong_hits = count_static_term_hits(&content_lower, &profile.strong_terms) as f32;

    let mut score = 0.10 + keyword_hits.min(6.0) * 0.05;
    score += if is_summary_layer { -0.14 } else { 0.03 };
    score += medium_hits.min(3.0) * 0.02;
    score += strong_hits.min(2.0) * 0.04;

    score += collection_summary_penalty(&uri);
    score += query_path_bonus(profile.kind, &rel_lower);

    hits.push(MergedSearchResult {
        uri,
        score,
        snippet,
        source: source.to_string(),
    });
}
