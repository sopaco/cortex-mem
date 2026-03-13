"use strict";
/**
 * Tool definitions for Cortex Memory plugin
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.toolSchemas = void 0;
// Tool input schemas (JSON Schema format)
exports.toolSchemas = {
    cortex_search: {
        name: 'cortex_search',
        description: `Layered semantic search across memory using L0/L1/L2 tiered retrieval.
Returns relevant memories ranked by relevance score.

Use this tool when you need to:
- Find past conversations or decisions
- Search for specific information across all sessions
- Discover related memories by semantic similarity`,
        inputSchema: {
            type: 'object',
            properties: {
                query: {
                    type: 'string',
                    description: 'The search query - can be natural language or keywords',
                },
                scope: {
                    type: 'string',
                    description: 'Optional session/thread ID to limit search scope',
                },
                limit: {
                    type: 'integer',
                    description: 'Maximum number of results to return (default: 10)',
                    default: 10,
                },
                min_score: {
                    type: 'number',
                    description: 'Minimum relevance score threshold (0-1, default: 0.6)',
                    default: 0.6,
                },
            },
            required: ['query'],
        },
    },
    cortex_recall: {
        name: 'cortex_recall',
        description: `Recall memories with layered detail levels (L0/L1/L2).

L0 (Abstract): ~100 tokens - Quick summary for relevance判断
L1 (Overview): ~2000 tokens - Key points and context
L2 (Full): Complete content - Use only when you need full details

Use this when you need more context than what cortex_search provides.`,
        inputSchema: {
            type: 'object',
            properties: {
                query: {
                    type: 'string',
                    description: 'The search query',
                },
                layers: {
                    type: 'array',
                    items: {
                        type: 'string',
                        enum: ['L0', 'L1', 'L2'],
                    },
                    description: 'Which detail layers to return (default: ["L0"])',
                    default: ['L0'],
                },
                scope: {
                    type: 'string',
                    description: 'Optional session/thread ID to limit search scope',
                },
                limit: {
                    type: 'integer',
                    description: 'Maximum number of results (default: 5)',
                    default: 5,
                },
            },
            required: ['query'],
        },
    },
    cortex_add_memory: {
        name: 'cortex_add_memory',
        description: `Add a message to memory for a specific session.
This stores the message and automatically triggers:
- Vector embedding for semantic search
- L0/L1 layer generation (async)

Use this to persist important information that should be searchable later.`,
        inputSchema: {
            type: 'object',
            properties: {
                content: {
                    type: 'string',
                    description: 'The content to store in memory',
                },
                role: {
                    type: 'string',
                    enum: ['user', 'assistant', 'system'],
                    description: 'Role of the message sender (default: user)',
                    default: 'user',
                },
                session_id: {
                    type: 'string',
                    description: 'Session/thread ID (uses default if not specified)',
                },
            },
            required: ['content'],
        },
    },
    cortex_list_sessions: {
        name: 'cortex_list_sessions',
        description: `List all memory sessions with their status.
Shows session IDs, message counts, and creation/update times.`,
        inputSchema: {
            type: 'object',
            properties: {},
        },
    },
    cortex_close_session: {
        name: 'cortex_close_session',
        description: `Close a memory session and trigger full memory extraction.

This triggers the complete memory processing pipeline:
1. Extracts structured memories (user preferences, entities, decisions) from conversation into the user/ directory
2. Generates complete L0/L1 layer summaries for the entire session
3. Indexes all extracted memories into the vector database
4. Marks the session as closed

Use this when:
- A conversation or task is complete and you want to consolidate memories
- You need user/ directory memories (preferences, entities) to be generated
- You want to ensure all L0/L1 summaries are up to date

Note: This is a potentially long-running operation (may take 30-60s due to LLM calls).`,
        inputSchema: {
            type: 'object',
            properties: {
                session_id: {
                    type: 'string',
                    description: 'Session/thread ID to close (uses default if not specified)',
                },
            },
        },
    },
};
//# sourceMappingURL=tools.js.map