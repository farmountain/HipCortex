// vscode-extension/scripts/fetch-bins.js
const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const PLATFORMS = [
  { name: 'linux', arch: 'amd64', asset: 'hipcortex-linux-amd64' },
  { name: 'linux', arch: 'arm64', asset: 'hipcortex-linux-arm64' },
  { name: 'darwin', arch: 'amd64', asset: 'hipcortex-macos-amd64' },
  { name: 'darwin', arch: 'arm64', asset: 'hipcortex-macos-arm64' },
  { name: 'win32', arch: 'amd64', asset: 'hipcortex-windows-amd64.exe' },
];

const BASE_DIR = path.join(__dirname, '..', 'server');

const MIN_BYTES = 1_000_000;

async function download(url, dest, redirects = 0) {
  if (redirects > 8) throw new Error('Too many redirects');
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      const status = res.statusCode || 0;
      if ([301, 302, 307, 308].includes(status)) {
        res.resume();
        const loc = res.headers.location;
        if (!loc) return reject(new Error('Redirect without location'));
        return download(loc, dest, redirects + 1).then(resolve).catch(reject);
      }
      if (status !== 200) {
        res.resume();
        return reject(new Error(`HTTP ${status} for ${url}`));
      }
      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on('finish', () => { file.close(); resolve(); });
      file.on('error', reject);
    }).on('error', reject);
  });
}

function isValidBinary(filePath) {
  try {
    const stats = fs.statSync(filePath);
    if (stats.size < MIN_BYTES) return false;
    const head = fs.readFileSync(filePath, { encoding: 'utf8', flag: 'r' }).slice(0, 32);
    return !head.includes('PLACEHOLDER') && !head.startsWith('<!');
  } catch {
    return false;
  }
}

async function main() {
  fs.mkdirSync(BASE_DIR, { recursive: true });
  for (const p of PLATFORMS) {
    const dir = path.join(BASE_DIR, p.name);
    fs.mkdirSync(dir, { recursive: true });
    const dest = path.join(dir, p.asset);
    if (isValidBinary(dest)) {
      console.log(`Skip ${p.asset} (valid)`);
      continue;
    }
    if (fs.existsSync(dest)) fs.unlinkSync(dest);
    const url = `https://github.com/farmountain/HipCortex/releases/latest/download/${p.asset}`;
    console.log(`Downloading ${p.asset}...`);
    await download(url, dest);
    if (!isValidBinary(dest)) {
      if (fs.existsSync(dest)) fs.unlinkSync(dest);
      throw new Error(`Downloaded ${p.asset} is invalid`);
    }
    if (p.name !== 'win32') {
      fs.chmodSync(dest, 0o755);
    }
    console.log(`  -> ${dest}`);
  }
  console.log('Done. Now run `npm run package` for a vsix that includes everything.');
}

main().catch(console.error);
