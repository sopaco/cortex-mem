# Troubleshooting Guide

Common MemClaw issues and their solutions.

## Installation Issues

### Platform Not Supported

**Symptoms**: "Platform not supported" error is displayed

**Solutions**:
- Confirm you are using macOS Apple Silicon (M1/M2/M3) or Windows x64
- Other platforms are not currently supported

### Plugin Installation Failed

**Symptoms**: `openclaw plugins install @memclaw/memclaw` fails

**Solutions**:
1. Check network connection
2. Confirm npm registry is accessible
3. Try using a proxy or mirror source

## Configuration Issues

### Invalid API Key

**Symptoms**: Search or memory operations return API errors

**Solutions**:
1. Verify `llmApiKey` and `embeddingApiKey` are correctly configured in OpenClaw plugin settings
2. Confirm API keys are valid and have sufficient quota
3. Confirm `llmApiBaseUrl` and `embeddingApiBaseUrl` are correct for your provider
4. Verify network connectivity to API endpoints

### Configuration Not Taking Effect

**Symptoms**: Service behavior doesn't change after modifying configuration

**Solutions**:
Open OpenClaw settings and verify MemClaw plugin configuration:

1. Open `openclaw.json` or navigate to Settings → Plugins → MemClaw
2. Ensure all required fields are correctly filled, especially the configuration sections related to LLM and Embedding.
3. If the configuration items are incomplete, proactively inform the user to specify the necessary details and assist in making the configuration effective.
4. Save changes and **restart OpenClaw Gateway** for changes to take effect

## Service Issues

### Service Won't Start

**Symptoms**: Service fails to start when plugin loads

**Solutions**:
1. Check if ports 6333, 6334, 8085 are occupied by other applications
2. Confirm API keys are configured in OpenClaw plugin settings
3. Check OpenClaw logs for detailed error messages

### Service Unreachable

**Symptoms**: Tool calls return connection errors

**Solutions**:
1. Confirm OpenClaw has been restarted and plugin is loaded
2. Check if `autoStartServices` configuration is set to `true` (default)
3. Verify firewall allows local connections on these ports

## Usage Issues

### No Search Results

**Symptoms**: `cortex_search` returns empty results

**Solutions**:
1. Run `cortex_list_sessions` to verify sessions exist
2. Lower `min_score` threshold (e.g., from 0.6 to 0.4)
3. Try different query terms or synonyms
4. Confirm that `cortex_add_memory` or `cortex_close_session` has been called previously to store memories

### Memory Extraction Failed

**Symptoms**: `cortex_close_session` fails or produces incomplete results

**Solutions**:
1. Verify LLM API configuration is correct
2. Check if API quota is sufficient
3. Check OpenClaw logs for detailed error messages

### Migration Failed

**Symptoms**: `cortex_migrate` fails to execute

**Solutions**:
1. Confirm OpenClaw workspace is located at `~/.openclaw/workspace`
2. Confirm memory files exist at `~/.openclaw/workspace/memory/`
3. Verify file permissions are correct

## Data Issues

### Data Location

MemClaw data storage locations:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/memclaw` |
| Windows | `%LOCALAPPDATA%\memclaw` |
| Linux | `~/.local/share/memclaw` |

### Data Safety

- **Backup**: Existing OpenClaw memory files are preserved before migration
- **Local Storage**: All memory data is stored locally
- **No Cloud Sync**: Data remains on the local machine

## Error Messages Reference

| Error Message | Possible Cause | Solution |
|---------------|----------------|----------|
| `Service not running` | Service not started | Restart OpenClaw or enable `autoStartServices` |
| `API error: 401` | Invalid API key | Check API key configuration |
| `API error: 429` | Rate limit exceeded | Wait and retry, or upgrade API plan |
| `Connection refused` | Service unreachable | Check port usage and service status |
| `No sessions found` | No memory data | Use `cortex_add_memory` to add memories |

## Getting Help

If the above solutions don't resolve your issue:

1. Check OpenClaw logs for detailed error messages
2. Submit an issue report at [GitHub Issues](https://github.com/sopaco/cortex-mem/issues)
3. Provide the following information:
   - Operating system and version
   - OpenClaw version
   - MemClaw plugin version
   - Relevant log snippets
   - Steps to reproduce
