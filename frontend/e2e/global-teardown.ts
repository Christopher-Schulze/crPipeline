import { execSync } from 'child_process';
import path from 'path';

export default async function globalTeardown() {
  const root = path.join(__dirname, '..', '..');
  execSync('docker compose down', { cwd: root, stdio: 'inherit' });
}
