import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { startMockLlm } from './mock-llm.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const workspace = path.join(root, 'artifacts/e2e-workspace');
const stateDir = path.join(root, 'artifacts/e2e-state');
const codeLog = path.join(root, 'artifacts/fake-code.log');
const screenshotDir = path.join(root, 'artifacts/e2e');
const port = 4390;
const mockPort = 4391;

fs.rmSync(workspace, { recursive: true, force: true });
fs.rmSync(stateDir, { recursive: true, force: true });
fs.mkdirSync(screenshotDir, { recursive: true });
fs.writeFileSync(codeLog, '');

const mock = await startMockLlm(mockPort);

const server = spawn(path.join(root, 'target/debug/mediadesk-server'), ['--port', String(port), '--no-open'], {
  env: {
    ...process.env,
    MEDIADESK_WORKSPACE: workspace,
    MEDIADESK_STATE_DIR: stateDir,
    MEDIADESK_CODE_BIN: path.join(root, 'scripts/fake-code.sh'),
    MEDIADESK_CODE_LOG: codeLog,
    MEDIADESK_CLI_BIN: path.join(root, 'scripts/mock-cli.mjs'),
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});

server.stdout.on('data', (chunk) => process.stdout.write(`[mediadesk] ${chunk}`));
server.stderr.on('data', (chunk) => process.stderr.write(`[mediadesk] ${chunk}`));
server.on('exit', (code) => {
  console.error(`[e2e] server exited (${code})`);
  mock.close();
  process.exit(code ?? 1);
});

async function waitForServer() {
  for (let i = 0; i < 150; i += 1) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/api/health`);
      if (response.ok) return;
    } catch {
      // retry
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('MediaDesk server did not become ready');
}

await waitForServer();
console.log(`[e2e] MediaDesk ready on http://127.0.0.1:${port}`);
console.log(`[e2e] mock LLM ready on http://127.0.0.1:${mockPort}/v1`);
console.log(`[e2e] workspace ${workspace}`);

function shutdown() {
  try {
    server.kill('SIGTERM');
  } catch {
    // ignore
  }
  try {
    mock.close();
  } catch {
    // ignore
  }
}

process.on('SIGTERM', () => {
  shutdown();
  process.exit(0);
});
process.on('SIGINT', () => {
  shutdown();
  process.exit(0);
});
