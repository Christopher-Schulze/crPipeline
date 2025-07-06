import { execSync } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';
const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default async function globalTeardown() {
  const root = path.join(__dirname, '..', '..');
  execSync('docker compose down', { cwd: root, stdio: 'inherit' });
}
