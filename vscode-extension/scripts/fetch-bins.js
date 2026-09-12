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
const EXT_DIR = path.join(__dirname, '..');
const REPO_ROOT = path.join(EXT_DIR, '..');

// The version the published hipcortex-* assets were built from. Resolved from the repo's own
// source of truth rather than hardcoded, so bumping the crate version cannot leave a stale
// default behind.
function readExpectedVersion() {
  const override = (process.env.HIPCORTEX_SERVER_VERSION || '').trim();
  if (override) return override;
  try {
    const raw = fs.readFileSync(path.join(REPO_ROOT, 'VERSION'), 'utf8').trim();
    const m = raw.match(/^\d+\.\d+\.\d+/);
    if (m) return m[0];
  } catch {}
  try {
    const m = fs.readFileSync(path.join(REPO_ROOT, 'Cargo.toml'), 'utf8').match(/^version\s*=\s*"([^"]+)"/m);
    if (m) return m[1];
  } catch {}
  try {
    return JSON.parse(fs.readFileSync(path.join(EXT_DIR, 'package.json'), 'utf8')).version;
  } catch {}
  throw new Error('cannot determine expected server version; set HIPCORTEX_SERVER_VERSION');
}

const EXPECTED_VERSION = readExpectedVersion();
// Pin to the release that matches EXPECTED_VERSION, not to a literal that goes stale on bump.
const RELEASE_TAG = process.env.HIPCORTEX_RELEASE_TAG || `v${EXPECTED_VERSION}`;
const FORCE =
  process.env.FORCE_FETCH_BINS === '1' ||
  process.argv.includes('--force');
const CHECK = process.argv.includes('--check');

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

/**
 * Does this file carry the expected server version as a literal?
 *
 * Size is not evidence of freshness: a stale 3.5.0 server is 8 MB and starts with "MZ", so a
 * size-only check happily stages it — and a VSIX labelled 3.10.0 then bundles a server that
 * predates the routes it is supposed to serve.
 */
function binaryHasVersion(filePath, expected = EXPECTED_VERSION) {
  try {
    // latin1 is byte-preserving for ASCII needles and skips UTF-8 decoding on ~8 MB files.
    return fs.readFileSync(filePath, 'latin1').includes(expected);
  } catch {
    return false;
  }
}

function isValidBinary(filePath, expected = EXPECTED_VERSION) {
  try {
    const stats = fs.statSync(filePath);
    if (stats.size < MIN_BYTES) return false;
    const head = fs.readFileSync(filePath, { encoding: 'utf8', flag: 'r' }).slice(0, 32);
    if (head.includes('PLACEHOLDER') || head.startsWith('<!')) return false;
    return binaryHasVersion(filePath, expected);
  } catch {
    return false;
  }
}

async function main() {
  if (CHECK) {
    // Gate for anything about to package the staged tree: is every binary the version the
    // extension claims to bundle? Exit non-zero instead of shipping a mismatch.
    let stale = 0;
    for (const p of PLATFORMS) {
      const dest = path.join(BASE_DIR, p.name, p.asset);
      const ok = isValidBinary(dest);
      if (!ok) stale++;
      const state = ok ? 'ok' : fs.existsSync(dest) ? 'STALE' : 'absent';
      console.log(`${state.padEnd(7)} ${p.name}/${p.asset}`);
    }
    if (stale) {
      throw new Error(`${stale} staged server binar${stale === 1 ? 'y is' : 'ies are'} not v${EXPECTED_VERSION}; run without --check to fetch them`);
    }
    console.log(`All staged server binaries are v${EXPECTED_VERSION}.`);
    return;
  }

  fs.mkdirSync(BASE_DIR, { recursive: true });
  for (const p of PLATFORMS) {
    const dir = path.join(BASE_DIR, p.name);
    fs.mkdirSync(dir, { recursive: true });
    const dest = path.join(dir, p.asset);
    if (!FORCE && isValidBinary(dest)) {
      console.log(`Skip ${p.asset} (v${EXPECTED_VERSION} present; set FORCE_FETCH_BINS=1 to refresh)`);
      continue;
    }
    if (fs.existsSync(dest)) {
      fs.unlinkSync(dest);
      console.log(
        FORCE
          ? `Force-refresh ${p.asset}`
          : `Replacing ${p.asset}: staged copy is not a usable v${EXPECTED_VERSION} binary`
      );
    }
    const url = `https://github.com/farmountain/HipCortex/releases/download/${RELEASE_TAG}/${p.asset}`;
    console.log(`Downloading ${p.asset} from ${RELEASE_TAG}...`);
    await download(url, dest);
    if (!isValidBinary(dest)) {
      if (fs.existsSync(dest)) fs.unlinkSync(dest);
      throw new Error(`Downloaded ${p.asset} is not a v${EXPECTED_VERSION} server binary`);
    }
    if (p.name !== 'win32') {
      fs.chmodSync(dest, 0o755);
    }
    console.log(`  -> ${dest} (${fs.statSync(dest).size} bytes)`);
  }
  console.log(
    `Done — v${EXPECTED_VERSION} server binaries staged. Now run \`npm run package\` for a vsix that includes everything.`
  );
}

// Exported for unit tests; the CLI only runs when this file is invoked directly.
if (require.main === module) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}

module.exports = {
  EXPECTED_VERSION,
  RELEASE_TAG,
  binaryHasVersion,
  isValidBinary,
  readExpectedVersion,
};
