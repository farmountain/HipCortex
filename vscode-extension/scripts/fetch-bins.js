// vscode-extension/scripts/fetch-bins.js
// Download platform server binaries into vscode-extension/server/.
// FORCE_FETCH_BINS=1 (or --force) always re-downloads so VSIX never ships stale bins.
const https = require('https');
const fs = require('fs');
const path = require('path');

const PLATFORMS = [
  { name: 'linux', arch: 'amd64', asset: 'hipcortex-linux-amd64' },
  { name: 'linux', arch: 'arm64', asset: 'hipcortex-linux-arm64' },
  { name: 'darwin', arch: 'amd64', asset: 'hipcortex-macos-amd64' },
  { name: 'darwin', arch: 'arm64', asset: 'hipcortex-macos-arm64' },
  { name: 'win32', arch: 'amd64', asset: 'hipcortex-windows-amd64.exe' },
];

const BASE_DIR = path.join(__dirname, '..', 'server');
const MIN_BYTES = 1_000_000;
// Pin to release that matches EXPECTED_SERVER_VERSION / crate 0.5.0 public ships.
const RELEASE_TAG = process.env.HIPCORTEX_RELEASE_TAG || 'v0.5.0';
const FORCE =
  process.env.FORCE_FETCH_BINS === '1' ||
  process.argv.includes('--force');

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
    if (!FORCE && isValidBinary(dest)) {
      console.log(`Skip ${p.asset} (valid; set FORCE_FETCH_BINS=1 to refresh)`);
      continue;
    }
    if (fs.existsSync(dest)) {
      fs.unlinkSync(dest);
      if (FORCE) console.log(`Force-refresh ${p.asset}`);
    }
    const url = `https://github.com/farmountain/HipCortex/releases/download/${RELEASE_TAG}/${p.asset}`;
    console.log(`Downloading ${p.asset} from ${RELEASE_TAG}...`);
    await download(url, dest);
    if (!isValidBinary(dest)) {
      if (fs.existsSync(dest)) fs.unlinkSync(dest);
      throw new Error(`Downloaded ${p.asset} is invalid`);
    }
    if (p.name !== 'win32') {
      fs.chmodSync(dest, 0o755);
    }
    console.log(`  -> ${dest} (${fs.statSync(dest).size} bytes)`);
  }
  console.log('Done. Now run `npm run package` for a vsix that includes everything.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
