/**
 * Aether Agent SDK - Web/WASM
 *
 * 在浏览器中原生运行 Aether Agent，无需后端。
 *
 * @example
 * ```typescript
 * import { Aether } from '@aether/sdk';
 *
 * const agent = new Aether('deepseek', 'deepseek-v4-flash', 'sk-xxx');
 * const reply = await agent.chat('你好');
 * console.log(reply);
 * ```
 */

import init, { AetherWasm } from './wasm/agent_wasm';

let _initialized = false;

async function ensureInit() {
  if (!_initialized) {
    await init();
    _initialized = true;
  }
}

export class Aether {
  private inner: AetherWasm | null = null;
  private _provider: string;
  private _model: string;
  private _key: string;

  constructor(provider: string, model: string, apiKey?: string) {
    this._provider = provider;
    this._model = model;
    this._key = apiKey || '';
  }

  async chat(message: string): Promise<string> {
    await ensureInit();
    if (!this.inner) {
      this.inner = new AetherWasm(this._provider, this._model, this._key || null);
    }
    return this.inner.chat(message);
  }
}

export default Aether;
export { init } from './wasm/agent_wasm';
