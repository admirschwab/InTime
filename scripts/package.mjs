// Bundles the release exe and the uninstaller into release/InTime-windows.zip, the archive
// that installer/install.ps1 downloads. The name has no version so that
// https://github.com/<owner>/<repo>/releases/latest/download/InTime-windows.zip always works.
// Run via `npm run package` (builds the exe first).
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const { version } = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
const exe = join(root, 'src-tauri', 'target', 'release', 'intime.exe');
if (!existsSync(exe)) {
  console.error(`Nicht gefunden: ${exe}\nZuerst "npm run tauri build" ausführen.`);
  process.exit(1);
}

const releaseDir = join(root, 'release');
const stageDir = join(releaseDir, 'InTime-windows');
const zip = join(releaseDir, 'InTime-windows.zip');
rmSync(releaseDir, { recursive: true, force: true });
mkdirSync(stageDir, { recursive: true });

copyFileSync(exe, join(stageDir, 'InTime.exe'));
copyFileSync(join(root, 'installer', 'uninstall.ps1'), join(stageDir, 'uninstall.ps1'));
writeFileSync(join(stageDir, 'version.txt'), `${version}\r\n`);

execFileSync(
  'powershell.exe',
  ['-NoProfile', '-Command', `Compress-Archive -Path '${stageDir}\\*' -DestinationPath '${zip}' -Force`],
  { stdio: 'inherit' },
);
console.log(`\nInTime ${version}: ${zip}`);
