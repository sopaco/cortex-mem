List directory contents to browse the memory space like a virtual filesystem.

This allows you to explore the hierarchical structure of memories:
- cortex://session - List all sessions
- cortex://session/{session_id} - Browse a specific session's contents
- cortex://session/{session_id}/timeline - View timeline messages
- cortex://user - View user-level memories (preferences, entities, goals)
- cortex://user/{user_id}/preferences - View user preferences (extracted memories)
- cortex://user/{user_id}/entities - View user entities (people, projects, etc.)
- cortex://agent - View agent-level memories
- cortex://agent/{agent_id}/cases - View agent problem-solution cases

**Parameters:**
- recursive: List all subdirectories recursively
- include_abstracts: Show L0 abstracts for each file (for quick preview)

Use this when:
- Semantic search doesn't find what you need
- You want to understand the overall memory layout
- You need to manually navigate to find specific information
