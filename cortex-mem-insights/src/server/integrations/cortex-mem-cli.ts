import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

// Cortex-mem-cli 集成客户端
export class CortexMemCliClient {
  private cliPath: string;
  
  constructor(cliPath: string = 'cortex-mem-cli') {
    this.cliPath = cliPath;
  }
  
  // 执行优化命令
  async optimize(params?: {
    memory_type?: string;
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    similarity_threshold?: number;
    dry_run?: boolean;
    verbose?: boolean;
  }): Promise<{
    success: boolean;
    message: string;
    data?: any;
    error?: string;
  }> {
    try {
      // 构建命令参数
      const args: string[] = ['optimize'];
      
      if (params?.memory_type) {
        args.push('--memory-type', params.memory_type);
      }
      
      if (params?.user_id) {
        args.push('--user-id', params.user_id);
      }
      
      if (params?.agent_id) {
        args.push('--agent-id', params.agent_id);
      }
      
      if (params?.run_id) {
        args.push('--run-id', params.run_id);
      }
      
      if (params?.actor_id) {
        args.push('--actor-id', params.actor_id);
      }
      
      if (params?.similarity_threshold) {
        args.push('--similarity-threshold', params.similarity_threshold.toString());
      }
      
      if (params?.dry_run) {
        args.push('--dry-run');
      }
      
      if (params?.verbose) {
        args.push('--verbose');
      }
      
      // 执行命令
      const command = `${this.cliPath} ${args.join(' ')}`;
      console.log('Executing command:', command);
      
      const { stdout, stderr } = await execAsync(command);
      
      if (stderr && !stderr.includes('warning')) {
        console.error('CLI stderr:', stderr);
      }
      
      // 解析输出
      let data: any = null;
      let message = 'Optimization completed successfully';
      
      try {
        // 尝试解析JSON输出
        const jsonMatch = stdout.match(/\{[\s\S]*\}/);
        if (jsonMatch) {
          data = JSON.parse(jsonMatch[0]);
          message = data.message || message;
        } else {
          // 如果没有JSON，使用原始输出
          data = { output: stdout.trim() };
        }
      } catch (parseError) {
        // 如果解析失败，使用原始输出
        data = { output: stdout.trim() };
      }
      
      return {
        success: true,
        message,
        data,
      };
    } catch (error) {
      console.error('Optimize command error:', error);
      
      let errorMessage = 'Failed to execute optimize command';
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      
      return {
        success: false,
        message: errorMessage,
        error: errorMessage,
      };
    }
  }
  
  // 列出记忆
  async list(params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
    format?: 'json' | 'table' | 'csv';
  }): Promise<{
    success: boolean;
    message: string;
    data?: any;
    error?: string;
  }> {
    try {
      const args: string[] = ['list'];
      
      if (params?.user_id) {
        args.push('--user-id', params.user_id);
      }
      
      if (params?.agent_id) {
        args.push('--agent-id', params.agent_id);
      }
      
      if (params?.run_id) {
        args.push('--run-id', params.run_id);
      }
      
      if (params?.actor_id) {
        args.push('--actor-id', params.actor_id);
      }
      
      if (params?.memory_type) {
        args.push('--memory-type', params.memory_type);
      }
      
      if (params?.limit) {
        args.push('--limit', params.limit.toString());
      }
      
      if (params?.format) {
        args.push('--format', params.format);
      } else {
        args.push('--format', 'json');
      }
      
      const command = `${this.cliPath} ${args.join(' ')}`;
      console.log('Executing command:', command);
      
      const { stdout, stderr } = await execAsync(command);
      
      if (stderr && !stderr.includes('warning')) {
        console.error('CLI stderr:', stderr);
      }
      
      let data: any = null;
      let message = 'List command completed';
      
      try {
        if (params?.format === 'json' || !params?.format) {
          data = JSON.parse(stdout);
          message = `Found ${data.total || data.length || 0} memories`;
        } else {
          data = { output: stdout.trim() };
        }
      } catch (parseError) {
        data = { output: stdout.trim() };
      }
      
      return {
        success: true,
        message,
        data,
      };
    } catch (error) {
      console.error('List command error:', error);
      
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Failed to execute list command',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
  
  // 搜索记忆
  async search(query: string, params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
    similarity_threshold?: number;
    format?: 'json' | 'table' | 'csv';
  }): Promise<{
    success: boolean;
    message: string;
    data?: any;
    error?: string;
  }> {
    try {
      const args: string[] = ['search', query];
      
      if (params?.user_id) {
        args.push('--user-id', params.user_id);
      }
      
      if (params?.agent_id) {
        args.push('--agent-id', params.agent_id);
      }
      
      if (params?.run_id) {
        args.push('--run-id', params.run_id);
      }
      
      if (params?.actor_id) {
        args.push('--actor-id', params.actor_id);
      }
      
      if (params?.memory_type) {
        args.push('--memory-type', params.memory_type);
      }
      
      if (params?.limit) {
        args.push('--limit', params.limit.toString());
      }
      
      if (params?.similarity_threshold) {
        args.push('--similarity-threshold', params.similarity_threshold.toString());
      }
      
      if (params?.format) {
        args.push('--format', params.format);
      } else {
        args.push('--format', 'json');
      }
      
      const command = `${this.cliPath} ${args.join(' ')}`;
      console.log('Executing command:', command);
      
      const { stdout, stderr } = await execAsync(command);
      
      if (stderr && !stderr.includes('warning')) {
        console.error('CLI stderr:', stderr);
      }
      
      let data: any = null;
      let message = 'Search completed';
      
      try {
        if (params?.format === 'json' || !params?.format) {
          data = JSON.parse(stdout);
          message = `Found ${data.total || data.results?.length || 0} results`;
        } else {
          data = { output: stdout.trim() };
        }
      } catch (parseError) {
        data = { output: stdout.trim() };
      }
      
      return {
        success: true,
        message,
        data,
      };
    } catch (error) {
      console.error('Search command error:', error);
      
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Failed to execute search command',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
  
  // 添加记忆
  async add(content: string, params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    role?: string;
    memory_type?: string;
  }): Promise<{
    success: boolean;
    message: string;
    data?: any;
    error?: string;
  }> {
    try {
      const args: string[] = ['add', `"${content.replace(/"/g, '\\"')}"`];
      
      if (params?.user_id) {
        args.push('--user-id', params.user_id);
      }
      
      if (params?.agent_id) {
        args.push('--agent-id', params.agent_id);
      }
      
      if (params?.run_id) {
        args.push('--run-id', params.run_id);
      }
      
      if (params?.actor_id) {
        args.push('--actor-id', params.actor_id);
      }
      
      if (params?.role) {
        args.push('--role', params.role);
      }
      
      if (params?.memory_type) {
        args.push('--memory-type', params.memory_type);
      }
      
      const command = `${this.cliPath} ${args.join(' ')}`;
      console.log('Executing command:', command);
      
      const { stdout, stderr } = await execAsync(command);
      
      if (stderr && !stderr.includes('warning')) {
        console.error('CLI stderr:', stderr);
      }
      
      let data: any = null;
      let message = 'Memory added successfully';
      
      try {
        const jsonMatch = stdout.match(/\{[\s\S]*\}/);
        if (jsonMatch) {
          data = JSON.parse(jsonMatch[0]);
          message = data.message || message;
        } else {
          data = { output: stdout.trim() };
        }
      } catch (parseError) {
        data = { output: stdout.trim() };
      }
      
      return {
        success: true,
        message,
        data,
      };
    } catch (error) {
      console.error('Add command error:', error);
      
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Failed to execute add command',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
  
  // 删除记忆
  async delete(id: string): Promise<{
    success: boolean;
    message: string;
    data?: any;
    error?: string;
  }> {
    try {
      const command = `${this.cliPath} delete ${id}`;
      console.log('Executing command:', command);
      
      const { stdout, stderr } = await execAsync(command);
      
      if (stderr && !stderr.includes('warning')) {
        console.error('CLI stderr:', stderr);
      }
      
      let data: any = null;
      let message = 'Memory deleted successfully';
      
      try {
        const jsonMatch = stdout.match(/\{[\s\S]*\}/);
        if (jsonMatch) {
          data = JSON.parse(jsonMatch[0]);
          message = data.message || message;
        } else {
          data = { output: stdout.trim() };
        }
      } catch (parseError) {
        data = { output: stdout.trim() };
      }
      
      return {
        success: true,
        message,
        data,
      };
    } catch (error) {
      console.error('Delete command error:', error);
      
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Failed to execute delete command',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
  
  // 获取CLI版本
  async version(): Promise<{
    success: boolean;
    version?: string;
    error?: string;
  }> {
    try {
      const command = `${this.cliPath} --version`;
      const { stdout } = await execAsync(command);
      
      const versionMatch = stdout.match(/cortex-mem-cli\s+([\d.]+)/);
      if (versionMatch) {
        return {
          success: true,
          version: versionMatch[1],
        };
      }
      
      return {
        success: true,
        version: stdout.trim(),
      };
    } catch (error) {
      console.error('Version command error:', error);
      
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to get version',
      };
    }
  }
  
  // 检查CLI是否可用
  async checkAvailability(): Promise<{
    available: boolean;
    version?: string;
    error?: string;
  }> {
    try {
      const versionResult = await this.version();
      
      if (versionResult.success && versionResult.version) {
        return {
          available: true,
          version: versionResult.version,
        };
      }
      
      return {
        available: false,
        error: versionResult.error || 'CLI not found',
      };
    } catch (error) {
      return {
        available: false,
        error: error instanceof Error ? error.message : 'CLI check failed',
      };
    }
  }
}

// 创建默认客户端实例
export const cortexMemCli = new CortexMemCliClient();