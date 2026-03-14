/**
 * Cortex Memory Plugin for OpenClaw
 *
 * Provides layered semantic memory with L0/L1/L2 tiered retrieval.
 *
 * Installation:
 *   openclaw plugins install @cortex-mem-openclaw/openclaw-plugin
 *
 * Configuration (in openclaw.json):
 *   {
 *     "plugins": {
 *       "entries": {
 *         "cortex-mem-openclaw": {
 *           "enabled": true,
 *           "config": {
 *             "serviceUrl": "http://127.0.0.1:8085",
 *             "tenantId": "tenant_claw",
 *             "defaultSessionId": "default",
 *             "searchLimit": 10,
 *             "minScore": 0.6
 *           }
 *         }
 *       }
 *     }
 *   }
 */

import { CortexMemClient } from "./client.js";
import {
  toolSchemas,
  type CortexSearchInput,
  type CortexRecallInput,
  type CortexAddMemoryInput,
  type CortexCloseSessionInput,
} from "./tools.js";

// Plugin configuration
interface PluginConfig {
  serviceUrl?: string;
  defaultSessionId?: string;
  searchLimit?: number;
  minScore?: number;
  tenantId?: string;
}

// OpenClaw Plugin API types — aligned with OpenClawPluginApi in openclaw/src/plugins/types.ts
interface PluginLogger {
  debug?: (msg: string, ...args: unknown[]) => void;
  info: (msg: string, ...args: unknown[]) => void;
  warn: (msg: string, ...args: unknown[]) => void;
  error: (msg: string, ...args: unknown[]) => void;
}

interface ToolDefinition {
  name: string;
  description: string;
  /**
   * JSON Schema for tool inputs.
   * OpenClaw uses 'parameters', NOT 'inputSchema'.
   */
  parameters: object;
  /**
   * Tool execution function.
   * OpenClaw uses 'execute(_id, params)', NOT 'handler(args)'.
   */
  execute: (_id: string, params: Record<string, unknown>) => Promise<unknown>;
  /** Optional: mark tool as opt-in (not auto-enabled) */
  optional?: boolean;
}

// Matches OpenClawPluginApi interface
interface PluginAPI {
  /**
   * Plugin-specific configuration from openclaw.json
   * Access via api.pluginConfig, NOT api.getConfig()
   */
  pluginConfig?: Record<string, unknown>;
  registerTool(tool: ToolDefinition, opts?: { optional?: boolean }): void;
  logger: PluginLogger;
}

// Export plugin as a default function — matches OpenClaw's resolvePluginModuleExport behavior
export default function cortexMemPlugin(api: PluginAPI) {
  const config = (api.pluginConfig ?? {}) as PluginConfig;
  const serviceUrl = config.serviceUrl ?? "http://localhost:8085";
  const defaultSessionId = config.defaultSessionId ?? "default";
  const searchLimit = config.searchLimit ?? 10;
  const minScore = config.minScore ?? 0.6;
  const tenantId = config.tenantId ?? "tenant_claw";

  const client = new CortexMemClient(serviceUrl);

  api.logger.info("Cortex Memory plugin initializing...");
  api.logger.info(`Service URL: ${serviceUrl}`);
  api.logger.info(`Tenant ID: ${tenantId}`);

  // Switch to the configured tenant on startup
  fetch(`${serviceUrl}/api/v2/tenants/switch`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tenant_id: tenantId }),
  })
    .then((res) => {
      if (res.ok) {
        api.logger.info(`✅ Switched to tenant: ${tenantId}`);
      } else {
        api.logger.warn(`Failed to switch tenant (${res.status}): ${tenantId}`);
      }
    })
    .catch((err) => {
      api.logger.warn(
        `Tenant switch request failed: ${err instanceof Error ? err.message : String(err)}`,
      );
    });

  // Register cortex_search tool
  api.registerTool({
    name: toolSchemas.cortex_search.name,
    description: toolSchemas.cortex_search.description,
    parameters: toolSchemas.cortex_search.inputSchema,
    execute: async (_id: string, params: Record<string, unknown>) => {
      const input = params as unknown as CortexSearchInput;

      try {
        const results = await client.search({
          query: input.query,
          thread: input.scope,
          limit: input.limit ?? searchLimit,
          min_score: input.min_score ?? minScore,
        });

        const formattedResults = results
          .map(
            (r, i) =>
              `${i + 1}. [Score: ${r.score.toFixed(2)}] ${r.snippet}\n   URI: ${r.uri}`,
          )
          .join("\n\n");

        return {
          content: `Found ${results.length} results for "${input.query}":\n\n${formattedResults}`,
          results: results.map((r) => ({
            uri: r.uri,
            score: r.score,
            snippet: r.snippet,
          })),
          total: results.length,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        api.logger.error(`cortex_search failed: ${message}`);
        return { error: `Search failed: ${message}` };
      }
    },
  });

  // Register cortex_recall tool
  api.registerTool({
    name: toolSchemas.cortex_recall.name,
    description: toolSchemas.cortex_recall.description,
    parameters: toolSchemas.cortex_recall.inputSchema,
    execute: async (_id: string, params: Record<string, unknown>) => {
      const input = params as unknown as CortexRecallInput;

      try {
        const results = await client.recall(
          input.query,
          input.layers ?? ["L0"],
          input.scope,
          input.limit ?? 5,
        );

        const layerLabels: Record<string, string> = {
          L0: "Abstract",
          L1: "Overview",
          L2: "Full Content",
        };

        const requestedLayers = input.layers ?? ["L0"];

        const formattedResults = results
          .map((r, i) => {
            let content = `${i + 1}. [Score: ${r.score.toFixed(2)}] URI: ${r.uri}\n`;

            if (requestedLayers.includes("L0") && r.abstract) {
              content += `   [${layerLabels["L0"]}]: ${r.abstract}\n`;
            }
            if (requestedLayers.includes("L1") && r.overview) {
              content += `   [${layerLabels["L1"]}]: ${r.overview.substring(0, 500)}...\n`;
            }
            if (requestedLayers.includes("L2") && r.content) {
              content += `   [${layerLabels["L2"]}]: ${r.content.substring(0, 500)}...\n`;
            }

            return content;
          })
          .join("\n");

        return {
          content: `Recalled ${results.length} memories:\n\n${formattedResults}`,
          results,
          total: results.length,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        api.logger.error(`cortex_recall failed: ${message}`);
        return { error: `Recall failed: ${message}` };
      }
    },
  });

  // Register cortex_add_memory tool
  api.registerTool({
    name: toolSchemas.cortex_add_memory.name,
    description: toolSchemas.cortex_add_memory.description,
    parameters: toolSchemas.cortex_add_memory.inputSchema,
    execute: async (_id: string, params: Record<string, unknown>) => {
      const input = params as unknown as CortexAddMemoryInput;

      try {
        const sessionId = input.session_id ?? defaultSessionId;
        const result = await client.addMessage(sessionId, {
          role: input.role ?? "user",
          content: input.content,
        });

        return {
          content: `Memory stored successfully in session "${sessionId}".\nResult: ${result}`,
          success: true,
          message_uri: result,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        api.logger.error(`cortex_add_memory failed: ${message}`);
        return { error: `Failed to add memory: ${message}` };
      }
    },
  });

  // Register cortex_list_sessions tool
  api.registerTool({
    name: toolSchemas.cortex_list_sessions.name,
    description: toolSchemas.cortex_list_sessions.description,
    parameters: toolSchemas.cortex_list_sessions.inputSchema,
    execute: async (_id: string, _params: Record<string, unknown>) => {
      try {
        const sessions = await client.listSessions();

        if (sessions.length === 0) {
          return { content: "No sessions found." };
        }

        const formattedSessions = sessions
          .map((s, i) => {
            const created = new Date(s.created_at).toLocaleDateString();
            return `${i + 1}. ${s.thread_id} (${s.status}, ${s.message_count} messages, created ${created})`;
          })
          .join("\n");

        return {
          content: `Found ${sessions.length} sessions:\n\n${formattedSessions}`,
          sessions: sessions.map((s) => ({
            thread_id: s.thread_id,
            status: s.status,
            message_count: s.message_count,
            created_at: s.created_at,
          })),
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        api.logger.error(`cortex_list_sessions failed: ${message}`);
        return { error: `Failed to list sessions: ${message}` };
      }
    },
  });

  // Register cortex_close_session tool
  api.registerTool({
    name: toolSchemas.cortex_close_session.name,
    description: toolSchemas.cortex_close_session.description,
    parameters: toolSchemas.cortex_close_session.inputSchema,
    execute: async (_id: string, params: Record<string, unknown>) => {
      const input = params as unknown as CortexCloseSessionInput;

      try {
        const sessionId = input.session_id ?? defaultSessionId;
        const result = await client.closeSession(sessionId);

        return {
          content: `Session "${sessionId}" closed successfully.\nStatus: ${result.status}, Messages: ${result.message_count}\n\nMemory extraction pipeline triggered — user preferences, entities, and L0/L1 summaries will be generated asynchronously.`,
          success: true,
          session: {
            thread_id: result.thread_id,
            status: result.status,
            message_count: result.message_count,
          },
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        api.logger.error(`cortex_close_session failed: ${message}`);
        return { error: `Failed to close session: ${message}` };
      }
    },
  });

  api.logger.info("Cortex Memory plugin initialized successfully");

  return {
    id: "cortex-mem-openclaw",
    name: "Cortex Memory",
    version: "0.1.0",
  };
}

// Also support object export style (register method calls the default function above)
export const plugin = {
  id: "cortex-mem-openclaw",
  name: "Cortex Memory",
  version: "0.1.0",
  configSchema: {
    type: "object",
    properties: {
      serviceUrl: { type: "string", default: "http://localhost:8085" },
      defaultSessionId: { type: "string", default: "default" },
      searchLimit: { type: "integer", default: 10 },
      minScore: { type: "number", default: 0.6 },
      tenantId: { type: "string", default: "tenant_claw" },
    },
    required: ["serviceUrl"],
  },
  register(api: PluginAPI) {
    return cortexMemPlugin(api);
  },
};
