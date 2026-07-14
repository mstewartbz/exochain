import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const webSrcDir = path.dirname(fileURLToPath(import.meta.url));
const demoRoot = path.resolve(webSrcDir, '..', '..');

function readDemoFile(relativePath) {
  return fs.readFileSync(path.join(demoRoot, relativePath), 'utf8');
}

function gatewayDefaultPort() {
  const source = readDemoFile('services/gateway-api/src/index.js');
  const match = source.match(/const PORT = process\.env\.PORT \|\| (\d+);/);

  expect(match).not.toBeNull();

  return Number(match[1]);
}

function viteApiProxyPort() {
  const source = readDemoFile('web/vite.config.js');
  const match = source.match(/['"]\/api['"]:\s*['"]http:\/\/localhost:(\d+)['"]/);

  expect(match).not.toBeNull();

  return Number(match[1]);
}

describe('Vite API proxy', () => {
  it('routes /api to the gateway-api port used by local demo entrypoints', () => {
    const gatewayPort = gatewayDefaultPort();
    const devScript = readDemoFile('scripts/dev.sh');
    const compose = readDemoFile('infra/docker-compose.yml');
    const readme = readDemoFile('README.md');

    expect(viteApiProxyPort()).toBe(gatewayPort);
    expect(devScript).toContain(`PORT=${gatewayPort} node services/gateway-api/src/index.js`);
    expect(devScript).toContain(`gateway-api:       http://localhost:${gatewayPort}`);
    expect(compose).toContain(`ports: ["${gatewayPort}:${gatewayPort}"]`);
    expect(compose).toContain(`PORT: ${gatewayPort}`);
    expect(readme).toContain(`| gateway-api | ${gatewayPort} |`);
  });
});
