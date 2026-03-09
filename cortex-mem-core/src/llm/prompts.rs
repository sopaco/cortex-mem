/// Prompt templates for various LLM tasks
pub struct Prompts;

impl Prompts {
    /// Prompt for generating L0 abstract
    ///
    /// ~100 tokens for quick relevance checking and filtering
    pub fn abstract_generation(content: &str) -> String {
        format!(
            r#"Generate a concise abstract (~100 tokens maximum) for the following content.

Requirements:
- Stay within ~100 tokens limit
- Cover MULTIPLE key aspects when content is rich (who, what, key topics, important outcomes)
- Prioritize information breadth over depth - mention more topics rather than elaborating on one
- Use compact phrasing: "discussed X, Y, and Z" instead of long explanations
- For multi-topic content: list key themes briefly rather than focusing on just one
- Use clear, direct language
- Avoid filler words and unnecessary details
- **CRITICAL: Use the SAME LANGUAGE as the input content**
  - If content is in Chinese, write abstract in Chinese
  - If content is in English, write abstract in English
  - If content is in other languages, use that language
  - Preserve the original linguistic and cultural context

Content:
{}

Abstract (max 100 tokens, in the same language as the content):"#,
            content
        )
    }

    /// Prompt for generating L1 overview
    ///
    /// ~2K tokens, structured overview
    /// for decision-making and planning
    pub fn overview_generation(content: &str) -> String {
        format!(
            r#"Generate a structured overview (~500-2000 tokens) of the following content.

Structure your response as markdown with these sections:

## Summary
2-3 paragraph overview of the core content and its significance

## Core Topics
List 3-5 main themes or topics (bullet points)

## Key Points
List 5-10 important takeaways or insights (numbered or bullets)

## Entities
Important people, organizations, technologies, or concepts mentioned

## Context
Any relevant background, timeframe, or situational information

Requirements:
- Use clear markdown formatting
- Be comprehensive but concise
- Focus on information useful for understanding and decision-making
- Aim for ~500-2000 tokens total
- **CRITICAL: Use the SAME LANGUAGE as the input content**
  - If content is in Chinese, write overview in Chinese
  - If content is in English, write overview in English
  - If content is in other languages, use that language
  - Preserve cultural references and linguistic nuances

Content:
{}

Structured Overview (in the same language as the content):"#,
            content
        )
    }

    /// Prompt for memory extraction from conversation
    pub fn memory_extraction(conversation: &str) -> String {
        format!(
            r#"Analyze the following conversation and extract:

1. **Facts**: Factual information that was shared or discovered
2. **Decisions**: Decisions that were made during the conversation
3. **Action Items**: Tasks or next steps that were identified
4. **User Preferences**: Any preferences, habits, or patterns expressed by the user
5. **Agent Learnings**: Insights or lessons learned that could help in future interactions

Format your response as JSON with the following structure:
{{
  "facts": [{{ "content": "...", "confidence": 0.9 }}],
  "decisions": [{{ "description": "...", "rationale": "..." }}],
  "action_items": [{{ "description": "...", "priority": "high|medium|low" }}],
  "user_preferences": [{{ "category": "...", "content": "..." }}],
  "agent_learnings": [{{ "task_type": "...", "learned_approach": "...", "success_rate": 0.8 }}]
}}

Conversation:
{}

Extracted Memories (JSON):"#,
            conversation
        )
    }

    /// 统一查询意图分析 Prompt（一次 LLM 请求返回所有检索所需信息）
    ///
    /// 返回：改写查询、关键词、实体列表、意图类型、时间约束
    /// 支持中英文及混合语言查询
    pub fn unified_query_analysis(query: &str) -> String {
        // 截断保护：query 最多取前 500 个字符（使用 chars 保证 Unicode 安全）
        let safe_query: String = query.chars().take(500).collect();

        format!(
            r#"Analyze the following search query and return a JSON object with all fields filled.

## Output JSON Format
{{
  "rewritten_query": "expanded query for better vector retrieval (keep original meaning, add synonyms, max 80 chars)",
  "keywords": ["keyword1", "keyword2"],
  "entities": ["entity1", "entity2"],
  "intent_type": "entity_lookup|factual|temporal|relational|search|general",
  "time_constraint": {{ "start": "...", "end": "..." }}
}}

## Field Rules
- **rewritten_query**: Expand abbreviations and add relevant synonyms. If already clear, keep as-is. Max 80 chars.
- **keywords**: 2-5 most important search terms. Must be in the same language as the query.
- **entities**: Named entities (person names, place names, tool names, technology names). Empty array if none.
- **intent_type**: Choose ONE from:
  - `entity_lookup`: Query asks about a specific named entity ("王明是谁", "who is Alice", "React框架")
  - `factual`: Asking for a specific fact ("X是什么", "what is X", "how does X work")
  - `temporal`: Involves time reference ("最近", "上周", "recently", "last week", "yesterday")
  - `relational`: Comparison or relationship ("X vs Y", "X和Y的关系", "difference between X and Y")
  - `search`: Looking to find/list content ("查找", "列出", "find", "show me", "list all")
  - `general`: Everything else
- **time_constraint**: Set to `null` if no time reference in query. Otherwise fill start/end as descriptive strings.

## Query
{}

## Response (valid JSON only, no markdown, no explanation):"#,
            safe_query
        )
    }

    /// Prompt for abstract generation with optional entity preservation
    ///
    /// When `known_entities` is non-empty, the LLM is instructed to retain
    /// those entity names in the generated abstract.
    pub fn abstract_generation_with_entities(content: &str, known_entities: &[String]) -> String {
        // 截断保护：content 最多取前 8000 个字符
        let safe_content: String = content.chars().take(8000).collect();

        let entity_hint = if known_entities.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nIMPORTANT: The following named entities MUST appear verbatim in the abstract \
                if they are present in the content: {}",
                known_entities.join(", ")
            )
        };

        format!(
            r#"Generate a concise abstract (~100 tokens maximum) for the following content.

Requirements:
- Stay within ~100 tokens limit
- Cover MULTIPLE key aspects when content is rich (who, what, key topics, important outcomes)
- Prioritize information breadth over depth - mention more topics rather than elaborating on one
- Use compact phrasing: "discussed X, Y, and Z" instead of long explanations
- For multi-topic content: list key themes briefly rather than focusing on just one
- Use clear, direct language
- Avoid filler words and unnecessary details
- **CRITICAL: Use the SAME LANGUAGE as the input content**
  - If content is in Chinese, write abstract in Chinese
  - If content is in English, write abstract in English
  - If content is in other languages, use that language
  - Preserve the original linguistic and cultural context{}

Content:
{}

Abstract (max 100 tokens, in the same language as the content):"#,
            entity_hint, safe_content
        )
    }
}
