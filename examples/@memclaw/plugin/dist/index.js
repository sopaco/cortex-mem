'use strict';
/**
 * MemClaw - Layered Semantic Memory for OpenClaw
 *
 * Provides:
 * - L0/L1/L2 tiered memory retrieval
 * - Automatic service startup (Qdrant + cortex-mem-service)
 * - Migration from OpenClaw native memory
 *
 * Installation:
 *   openclaw plugins install memclaw
 *
 * Configuration (in openclaw.json):
 *   {
 *     "plugins": {
 *       "entries": {
 *         "memclaw": {
 *           "enabled": true,
 *           "config": {
 *             "serviceUrl": "http://127.0.0.1:8085",
 *             "tenantId": "tenant_claw",
 *             "autoStartServices": true
 *           }
 *         }
 *       }
 *     }
 *   }
 */
Object.defineProperty(exports, '__esModule', { value: true });
exports.plugin = void 0;
exports.default = memclawPlugin;
const plugin_impl_js_1 = require('./plugin-impl.js');
// Default export - main plugin function
function memclawPlugin(api) {
	return (0, plugin_impl_js_1.createPlugin)(api);
}
// Named export - object style registration
exports.plugin = {
	id: 'memclaw',
	name: 'MemClaw',
	version: '0.9.6',
	configSchema: {
		type: 'object',
		properties: {
			serviceUrl: { type: 'string', default: 'http://localhost:8085' },
			defaultSessionId: { type: 'string', default: 'default' },
			searchLimit: { type: 'integer', default: 10 },
			minScore: { type: 'number', default: 0.6 },
			tenantId: { type: 'string', default: 'tenant_claw' },
			autoStartServices: { type: 'boolean', default: true }
		},
		required: []
	},
	register(api) {
		return (0, plugin_impl_js_1.createPlugin)(api);
	}
};
//# sourceMappingURL=index.js.map
