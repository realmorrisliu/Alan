/**
 * Daemon Manager - 管理 agentd 进程的生命周期
 * 
 * 支持两种运行模式：
 * 1. 开发模式：从源码运行 (bun run src/index.tsx)
 * 2. 生产模式：从 ~/.alan/bin 运行
 * 
 * 配置文件路径：
 * - 生产模式: ~/.alan/config/agentd.toml
 * - 开发模式: 项目根目录下的 agentd.toml（如果存在）
 * 
 * 参考 pi-mono 的设计：TUI 是完整的交互工具，自动管理后端
 */

import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { homedir } from 'node:os';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * 获取配置文件路径
 */
function getConfigPath(): string | null {
  // 1. 环境变量指定
  if (process.env.ALAN_CONFIG_PATH) {
    return process.env.ALAN_CONFIG_PATH;
  }
  
  // 2. 生产模式：~/.alan/config/agentd.toml
  const prodConfig = join(homedir(), '.alan', 'config', 'agentd.toml');
  if (existsSync(prodConfig)) {
    return prodConfig;
  }
  
  // 3. 开发模式：检查项目根目录
  const rootDir = resolve(__dirname, '../../../');
  const devConfig = join(rootDir, 'agentd.toml');
  if (existsSync(devConfig)) {
    return devConfig;
  }
  
  return null;
}

/**
 * 检测运行模式
 */
function detectMode(): { mode: 'development' | 'production'; rootDir: string } {
  // 检查当前目录（clients/tui）下是否有 src/index.tsx（开发模式特征）
  const hasSrcIndex = existsSync(join(__dirname, '../src/index.tsx')) ||
                      existsSync(join(__dirname, 'src/index.tsx'));
  
  // 或者本文件是否在 src/ 目录下
  const isInSrcDir = __dirname.endsWith('/src') || __dirname.endsWith('\\src');
  
  if (hasSrcIndex || isInSrcDir) {
    // 开发模式：从 clients/tui/src/ 向上找项目根目录（Alan/）
    const rootDir = resolve(__dirname, '../../../');
    return { mode: 'development', rootDir };
  } else {
    // 生产模式：从可执行文件位置找
    const execDir = __dirname;
    return { mode: 'production', rootDir: execDir };
  }
}

export interface DaemonConfig {
  /** 绑定的端口，默认 8090 */
  port?: number;
  /** 主机地址，默认 127.0.0.1 */
  host?: string;
  /** 工作目录 */
  cwd?: string;
  /** 额外环境变量 */
  env?: Record<string, string>;
  /** 启动超时（毫秒） */
  startupTimeout?: number;
  /** 是否显示 agentd 输出（调试用） */
  verbose?: boolean;
}

export interface DaemonStatus {
  state: 'stopped' | 'starting' | 'running' | 'error';
  pid?: number;
  url: string;
  error?: string;
}

export class DaemonManager {
  private process: ChildProcess | null = null;
  private config: Required<DaemonConfig>;
  private status: DaemonStatus = { state: 'stopped', url: '' };
  private logBuffer: string[] = [];
  private maxLogBuffer = 100;

  constructor(config: DaemonConfig = {}) {
    this.config = {
      port: config.port ?? 8090,
      host: config.host ?? '127.0.0.1',
      cwd: config.cwd ?? process.cwd(),
      env: config.env ?? {},
      startupTimeout: config.startupTimeout ?? 10000,
      verbose: config.verbose ?? false,
    };
    this.status.url = `http://${this.config.host}:${this.config.port}`;
  }

  /**
   * 查找 agentd 可执行文件
   * 
   * 搜索顺序：
   * 1. 环境变量 ALAN_AGENTD_PATH
   * 2. 相对于本文件的路径（开发模式）
   * 3. 相对于当前工作目录的路径
   * 4. 生产模式路径
   * 5. 系统 PATH
   */
  private findAgentdBinary(): string | null {
    // 1. 环境变量（最高优先级）
    if (process.env.ALAN_AGENTD_PATH) {
      const envPath = resolve(process.env.ALAN_AGENTD_PATH);
      if (existsSync(envPath)) return envPath;
    }

    const platform = process.platform;
    const exeSuffix = platform === 'win32' ? '.exe' : '';

    // 2. 尝试从本文件位置推导项目根目录
    // daemon.ts 在 clients/tui/src/，向上三级到项目根（src -> tui -> clients -> root）
    const projectRootFromHere = resolve(__dirname, '../../../');
    const devPaths = [
      join(projectRootFromHere, 'target/release/agentd'),
      join(projectRootFromHere, 'target/debug/agentd'),
      join(projectRootFromHere, `target/release/agentd${exeSuffix}`),
      join(projectRootFromHere, `target/debug/agentd${exeSuffix}`),
    ];

    for (const path of devPaths) {
      if (existsSync(path)) return path;
    }

    // 3. 当前工作目录（用户可能在项目根目录运行）
    const cwdPaths = [
      join(process.cwd(), 'target/release/agentd'),
      join(process.cwd(), 'target/debug/agentd'),
      join(process.cwd(), `target/release/agentd${exeSuffix}`),
      join(process.cwd(), `target/debug/agentd${exeSuffix}`),
    ];

    for (const path of cwdPaths) {
      if (existsSync(path)) return path;
    }

    // 4. 生产模式：可执行文件同目录
    const prodPaths = [
      join(__dirname, 'agentd'),
      join(__dirname, `agentd${exeSuffix}`),
      join(__dirname, '../agentd'),
      join(__dirname, `../agentd${exeSuffix}`),
    ];

    for (const path of prodPaths) {
      if (existsSync(path)) return path;
    }

    // 5. 系统 PATH
    return 'agentd';
  }

  /**
   * 检查 agentd 是否已经在运行
   */
  async isRunning(): Promise<boolean> {
    try {
      const response = await fetch(`${this.status.url}/health`, {
        signal: AbortSignal.timeout(1000),
      });
      return response.ok;
    } catch {
      return false;
    }
  }

  /**
   * 等待 agentd 启动就绪
   */
  private async waitForReady(timeoutMs: number): Promise<void> {
    const startTime = Date.now();
    const checkInterval = 100;

    return new Promise((resolve, reject) => {
      const check = async () => {
        if (Date.now() - startTime > timeoutMs) {
          const logs = this.logBuffer.slice(-10).join('\n');
          reject(new Error(`agentd 启动超时。最近日志:\n${logs}`));
          return;
        }

        if (await this.isRunning()) {
          resolve();
          return;
        }

        setTimeout(check, checkInterval);
      };
      check();
    });
  }

  /**
   * 启动 agentd
   */
  async start(): Promise<DaemonStatus> {
    if (this.status.state === 'running') {
      return this.status;
    }

    if (this.status.state === 'starting') {
      throw new Error('agentd 正在启动中');
    }

    // 检查是否已经有其他 agentd 在运行
    if (await this.isRunning()) {
      this.status = { state: 'running', url: this.status.url };
      return this.status;
    }

    const binary = this.findAgentdBinary();
    if (!binary || binary === 'agentd') {
      const { mode } = detectMode();
      let errorMsg = '找不到 agentd 可执行文件。\n\n';
      
      if (mode === 'development') {
        const projectRoot = resolve(__dirname, '../../../');
        errorMsg += '开发模式解决方案:\n';
        errorMsg += `1. 在项目根目录编译 agentd:\n`;
        errorMsg += `   cd ${projectRoot}\n`;
        errorMsg += `   cargo build --release -p alan-agentd\n\n`;
        errorMsg += `2. 或者设置环境变量:\n`;
        errorMsg += `   ALAN_AGENTD_PATH=/path/to/agentd bun run src/index.tsx\n`;
      } else {
        errorMsg += '生产模式解决方案:\n';
        errorMsg += '1. 重新安装 alan:\n';
        errorMsg += '   just install\n\n';
        errorMsg += '2. 或者设置环境变量:\n';
        errorMsg += '   ALAN_AGENTD_PATH=/path/to/agentd\n';
      }
      
      throw new Error(errorMsg);
    }
    
    // 检查配置文件
    const configPath = getConfigPath();
    if (!configPath) {
      const { mode } = detectMode();
      let warnMsg = '\n⚠️  未找到配置文件。\n\n';
      
      if (mode === 'production') {
        warnMsg += '请创建配置文件:\n';
        warnMsg += `   vim ~/.alan/config/agentd.toml\n\n`;
        warnMsg += '或使用模板:\n';
        warnMsg += `   mkdir -p ~/.alan/config\n`;
        warnMsg += `   cp ~/.alan/config/agentd.toml.example ~/.alan/config/agentd.toml\n`;
      } else {
        warnMsg += '请创建配置文件（项目根目录或 ~/.alan/config/agentd.toml）\n';
      }
      
      // 作为警告但不阻止启动（agentd 可能有其他配置方式）
      if (this.config.verbose) {
        console.warn(warnMsg);
      }
    }

    this.status = { state: 'starting', url: this.status.url };
    this.logBuffer = [];

    return new Promise((resolve, reject) => {
      // 获取配置文件路径
      const configPath = getConfigPath();
      
      const env: Record<string, string> = {
        ...process.env,
        ...this.config.env,
        BIND_ADDRESS: `${this.config.host}:${this.config.port}`,
      };
      
      // 如果找到配置文件，设置环境变量
      if (configPath) {
        env.ALAN_CONFIG_PATH = configPath;
        if (this.config.verbose) {
          console.log(`[Daemon] Using config: ${configPath}`);
        }
      }

      const args: string[] = [];
      
      this.process = spawn(binary, args, {
        env,
        cwd: this.config.cwd,
        stdio: this.config.verbose ? 'inherit' : 'pipe',
      });

      if (!this.process.pid) {
        this.status = { state: 'error', url: this.status.url, error: '无法启动进程' };
        reject(new Error('无法启动 agentd 进程'));
        return;
      }

      this.status.pid = this.process.pid;

      // 捕获输出用于调试
      if (this.process.stdout && this.process.stderr) {
        this.process.stdout.on('data', (data: Buffer) => {
          const line = data.toString().trim();
          if (this.config.verbose) {
            console.log(`[agentd] ${line}`);
          }
          this.logBuffer.push(line);
          if (this.logBuffer.length > this.maxLogBuffer) {
            this.logBuffer.shift();
          }
        });

        this.process.stderr.on('data', (data: Buffer) => {
          const line = data.toString().trim();
          if (this.config.verbose) {
            console.error(`[agentd] ${line}`);
          }
          this.logBuffer.push(`[stderr] ${line}`);
          if (this.logBuffer.length > this.maxLogBuffer) {
            this.logBuffer.shift();
          }
        });
      }

      // 处理进程退出
      this.process.on('exit', (code, signal) => {
        if (this.status.state !== 'stopped') {
          this.status = {
            state: 'error',
            url: this.status.url,
            error: `agentd 意外退出 (code: ${code}, signal: ${signal})`,
          };
        }
      });

      // 等待启动完成
      this.waitForReady(this.config.startupTimeout)
        .then(() => {
          this.status = { state: 'running', pid: this.process?.pid, url: this.status.url };
          resolve(this.status);
        })
        .catch((error) => {
          const logs = this.logBuffer.slice(-10).join('\n');
          this.stop().catch(() => {}); // 清理失败的进程
          this.status = { state: 'error', url: this.status.url, error: error.message };
          
          // 增强错误信息
          let enhancedError = error.message;
          if (logs.includes('LLM') || logs.includes('Gemini') || logs.includes('OpenAI')) {
            enhancedError += '\n\n提示: 看起来 agentd 需要 LLM 配置。请设置以下环境变量之一:\n';
            enhancedError += '  - Gemini: LLM_PROVIDER=gemini GEMINI_PROJECT_ID=your-project\n';
            enhancedError += '  - OpenAI: LLM_PROVIDER=openai_compatible OPENAI_COMPAT_API_KEY=your-key\n';
            enhancedError += '  - Anthropic: LLM_PROVIDER=anthropic_compatible ANTHROPIC_COMPAT_API_KEY=your-key\n';
          }
          if (logs.length > 0 && !this.config.verbose) {
            enhancedError += `\n\n最近日志:\n${logs}`;
          }
          
          reject(new Error(enhancedError));
        });
    });
  }

  /**
   * 停止 agentd
   */
  async stop(): Promise<void> {
    if (!this.process || this.status.state === 'stopped') {
      this.status = { state: 'stopped', url: this.status.url };
      return;
    }

    return new Promise((resolve) => {
      const timeout = setTimeout(() => {
        // 强制终止
        this.process?.kill('SIGKILL');
      }, 5000);

      this.process?.once('exit', () => {
        clearTimeout(timeout);
        this.process = null;
        this.status = { state: 'stopped', url: this.status.url };
        resolve();
      });

      // 先尝试优雅终止
      this.process?.kill('SIGTERM');
    });
  }

  /**
   * 获取当前状态
   */
  getStatus(): DaemonStatus {
    return { ...this.status };
  }

  /**
   * 获取日志缓冲
   */
  getLogs(): string[] {
    return [...this.logBuffer];
  }
}

/**
 * 全局 DaemonManager 实例
 */
let globalDaemon: DaemonManager | null = null;

export function getDaemon(config?: DaemonConfig): DaemonManager {
  if (!globalDaemon) {
    globalDaemon = new DaemonManager(config);
  }
  return globalDaemon;
}

export async function ensureDaemon(config?: DaemonConfig): Promise<DaemonManager> {
  const daemon = getDaemon(config);
  
  if (daemon.getStatus().state !== 'running') {
    await daemon.start();
  }
  
  return daemon;
}

export async function stopGlobalDaemon(): Promise<void> {
  if (globalDaemon) {
    await globalDaemon.stop();
    globalDaemon = null;
  }
}
