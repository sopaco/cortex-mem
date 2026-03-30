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

## Timeline Events
List ALL dated events with exact timestamps. Format each as:
- [DATE/TIME]: Brief description of the event
If no dated events are mentioned, write: (none)

## Activities & Interests
List ALL specific activities, hobbies, sports, arts, outdoor activities, or interests mentioned:
- [PERSON]: activity/hobby
If none mentioned, write: (none)

## Context
Any relevant background, timeframe, or situational information

Requirements:
- Use clear markdown formatting
- Be comprehensive but concise
- **ALWAYS fill in Timeline Events and Activities & Interests sections** - these are critical for memory retrieval
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
            r#"Analyze the following conversation and extract structured memory information.

Extract ALL of the following categories that are present:

1. **Events** (with dates): Specific events, activities, or occurrences with their dates/times
   - Include: meetings, trips, races, classes, ceremonies, conferences, appointments
   - MUST include the exact date/time mentioned (e.g., "July 2, 2023", "last Sunday")
2. **Personal Info**: Identity facts about the people in the conversation
   - Include: career, job, profession, relationship status, identity, background, research topics
   - Include: what someone studied, investigated, or is working on
3. **Activities & Hobbies**: Specific activities, sports, hobbies, or interests
   - Include: pottery, camping, swimming, painting, hiking, cooking, yoga, etc.
   - Note if someone signed up for or enrolled in something
4. **Future Plans & Schedule**: Upcoming events, conferences, or scheduled activities
   - Include: events planned for specific future dates or months
5. **Relationships**: Information about relationships between people
6. **User Preferences**: Preferences, habits, or patterns expressed
7. **Facts**: Other factual information shared
8. **Agent Learnings**: Insights useful for future interactions

Format your response as JSON:
{{
  "events": [{{ "title": "...", "date": "exact date/time string", "description": "...", "participants": [] }}],
  "personal_info": [{{ "person": "...", "category": "career|relationship|identity|research|other", "content": "..." }}],
  "activities": [{{ "person": "...", "activity": "...", "context": "signed up|participates in|enjoys|etc" }}],
  "future_plans": [{{ "person": "...", "event": "...", "date": "...", "description": "..." }}],
  "relationships": [{{ "persons": ["...", "..."], "type": "...", "description": "..." }}],
  "facts": [{{ "content": "...", "confidence": 0.9 }}],
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
  "rewritten_query": "expanded query for better vector retrieval (keep original meaning, add synonyms, max 150 chars)",
  "keywords": ["keyword1", "keyword2"],
  "entities": ["entity1", "entity2"],
  "intent_type": "entity_lookup|factual|temporal|relational|search|general",
  "time_constraint": {{ "start": "...", "end": "..." }}
}}

## Field Rules
- **rewritten_query**: Aggressively expand the query with synonyms and related concepts for maximum vector recall. Max 150 chars.
  - For personal fact questions ("what did X do/research/study"): add domain synonyms, e.g. "Caroline research → Caroline studied investigated explored topic subject adoption agencies"
  - For date/time questions ("when did X happen"): expand with event keywords, e.g. "when pottery class → pottery class signup enrolled date schedule July"
  - For activity/hobby questions ("what activities/hobbies does X do"): add activity-type words, e.g. "Melanie activities → Melanie hobbies sports pottery camping swimming painting arts outdoor"
  - For relationship/status questions: add inference keywords, e.g. "relationship status → single dating breakup partner"
  - For conference/event questions: add scheduling words, e.g. "conference → conference event attend scheduled July 2023"
- **keywords**: 3-6 most important search terms including expanded terms. Must be in the same language as the query.
- **entities**: Named entities (person names, place names, tool names, technology names). Empty array if none.
- **intent_type**: Choose ONE from:
  - `entity_lookup`: Query asks about a specific named entity ("王明是谁", "who is Alice", "React框架")
  - `factual`: Asking for a specific personal fact about someone ("what did X research", "what is X's job", "what did X do")
  - `temporal`: Involves time reference ("最近", "上周", "recently", "last week", "when did", "what date")
  - `relational`: Comparison or relationship ("X vs Y", "X和Y的关系", "difference between X and Y", "relationship status")
  - `search`: Looking to find/list content ("查找", "列出", "find", "show me", "list all", "what activities", "what hobbies")
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
