/**
 * Regression tests for the packaging-time server-version gate.
 *
 * Context: `npm run fetch-bins` used to accept any staged file that was big enough and did not
 * look like an HTML error page. A stale hipcortex 3.5.0 server satisfied both checks, so a VSIX
 * labelled 3.10.0 bundled a 3.5.0 server — which ignores `record_type` on POST /memory/add and
 * therefore cannot write a durable record at all. These tests pin the version check that closes
 * that window.
 */
const fs = require('fs');
const os = require('os');
const path = require('path');

// eslint-disable-next-line @typescript-eslint/no-var-requires
const fetchBins = require('../../scripts/fetch-bins.js');

describe('fetch-bins version gate', () => {
    let dir: string;

    beforeAll(() => {
        dir = fs.mkdtempSync(path.join(os.tmpdir(), 'fetch-bins-'));
    });

    afterAll(() => {
        fs.rmSync(dir, { recursive: true, force: true });
    });

    /** A file that looks like a real staging artefact: large, and starting with the MZ header. */
    function stagedBinary(name: string, body: string, size = 1_100_000): string {
        const target = path.join(dir, name);
        const pad = 'x'.repeat(Math.max(0, size - body.length - 2));
        fs.writeFileSync(target, 'MZ' + body + pad);
        return target;
    }

    /** The version the repository is currently stamped with, read the same way the script reads it. */
    function repoVersion(): string {
        const raw = fs.readFileSync(path.join(__dirname, '..', '..', '..', 'VERSION'), 'utf8');
        return raw.trim().match(/^\d+\.\d+\.\d+/)![0];
    }

    test('expected version comes from the repo VERSION file, not a hardcoded literal', () => {
        expect(fetchBins.EXPECTED_VERSION).toBe(repoVersion());
    });

    test('release tag is derived from the expected version', () => {
        expect(fetchBins.RELEASE_TAG).toBe(`v${fetchBins.EXPECTED_VERSION}`);
    });

    test('a stale but large binary is rejected (the 3.5.0-in-a-3.10.0-VSIX regression)', () => {
        const stale = stagedBinary('stale.exe', 'hipcortex server version 3.5.0');
        expect(fetchBins.binaryHasVersion(stale, '3.10.0')).toBe(false);
        expect(fetchBins.isValidBinary(stale, '3.10.0')).toBe(false);
    });

    test('a binary carrying the expected version is accepted', () => {
        const fresh = stagedBinary('fresh.exe', 'hipcortex server version 3.10.0');
        expect(fetchBins.binaryHasVersion(fresh, '3.10.0')).toBe(true);
        expect(fetchBins.isValidBinary(fresh, '3.10.0')).toBe(true);
    });

    test('the default expected version is the one the repo is stamped with', () => {
        const fresh = stagedBinary('fresh-default.exe', `hipcortex server version ${repoVersion()}`);
        expect(fetchBins.isValidBinary(fresh)).toBe(true);
    });

    test('an oversized HTML error page is still rejected', () => {
        const target = path.join(dir, 'error.exe');
        fs.writeFileSync(target, '<!DOCTYPE html>' + 'x'.repeat(1_100_000));
        expect(fetchBins.isValidBinary(target)).toBe(false);
    });

    test('an undersized file carrying the right version is still rejected', () => {
        const target = path.join(dir, 'truncated.exe');
        fs.writeFileSync(target, `MZhipcortex ${repoVersion()}`);
        expect(fetchBins.isValidBinary(target)).toBe(false);
    });

    test('a missing file is not a valid binary', () => {
        expect(fetchBins.isValidBinary(path.join(dir, 'nope.exe'))).toBe(false);
    });
});
