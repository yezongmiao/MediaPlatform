import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const apiPort = process.env.MEDIADESK_PORT ?? '4317';
const apiBase = `http://127.0.0.1:${apiPort}`;
let shuttingDown = false;

const server = spawn('cargo', ['run', '-p', 'mediadesk-server', '--', '--port', apiPort, '--no-open'], {
  cwd: root,
  stdio: 'inherit',
});

let vite;

async function waitForApi() {
  for (let i = 0; i < 240; i += 1) {
    try {
      const res = await fetch(`${apiBase}/api/health`);
      if (res.ok) return true;
    } catch {
      // retry
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  return false;
}

function shutdown(code = 0) {
  if (shuttingDown) return;
  shuttingDown = true;
  vite?.kill('SIGTERM');
  server.kill('SIGTERM');
  setTimeout(() => process.exit(code), 250);
}

server.on('exit', (code) => {
  if (!shuttingDown) {
    console.error('[dev] API server exited, stopping Vite…');
    shutdown(code ?? 1);
  }
});

process.on('SIGINT', () => shutdown(0));
process.on('SIGTERM', () => shutdown(0));

if (!(await waitForApi())) {
  console.error('[dev] API server did not become ready');
  shutdown(1);
} else {
  console.log(`[dev] API ready at ${apiBase}`);
  console.log('[dev] Vite dev server: http://127.0.0.1:5173');
  vite = spawn(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js')], {
    cwd: root,
    stdio: 'inherit',
    env: { ...process.env, MEDIADESK_API: apiBase },
  });
  vite.on('exit', (code) => {
    if (!shuttingDown) shutdown(code ?? 0);
  });
}
