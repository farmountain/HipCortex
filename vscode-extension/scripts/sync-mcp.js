// Syncs sdk/mcp/server.py into server/mcp_server.py before VSIX packaging.
const fs   = require('fs');
const path = require('path');

const src = path.join(__dirname, '..', '..', 'sdk', 'mcp', 'server.py');
const dst = path.join(__dirname, '..', 'server', 'mcp_server.py');

fs.copyFileSync(src, dst);
console.log(`sync-mcp: ${src} → ${dst}`);
