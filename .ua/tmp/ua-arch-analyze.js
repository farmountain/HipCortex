#!/usr/bin/env node
// Architecture analysis script for HipCortex knowledge graph
// Handles the actual node structure: nodes use 'file' property (not 'filePath'),
// and sub-file nodes (concept/class/function) are excluded from layer assignment.

const fs = require('fs');

const inputPath = process.argv[2];
const outputPath = process.argv[3];

if (!inputPath || !outputPath) {
  console.error('Usage: node ua-arch-analyze.js <input.json> <output.json>');
  process.exit(1);
}

const kg = JSON.parse(fs.readFileSync(inputPath, 'utf8'));
const allNodes = kg.nodes || [];
const allEdges = kg.edges || [];

// File-level node types (exclude concept/class/function which are sub-file)
const FILE_LEVEL_TYPES = new Set(['service','config','pipeline','document','resource','schema','file']);

const fileNodes = allNodes.filter(n => FILE_LEVEL_TYPES.has(n.type));
const importEdges = allEdges.filter(e => e.type === 'imports');

console.error(`File-level nodes: ${fileNodes.length}, Import edges: ${importEdges.length}, All edges: ${allEdges.length}`);

// Helper: get the file path for a node
function getFilePath(node) {
  return node.file || node.filePath || node.id;
}

// Helper: get directory group from a file path
function getDirGroup(filePath) {
  if (!filePath) return 'root';
  // Strip known prefixes like "file:", "file:file:", "document:document:", etc.
  let p = filePath.replace(/^(file:file:|file:file:|document:document:|config:config:|schema:schema:|file:|document:|config:|service:|pipeline:|resource:|schema:)/, '');

  const parts = p.split('/');
  if (parts.length === 1) return 'root';

  // Top-level directory
  const top = parts[0];

  // Special subdirectory groupings
  if (top === 'src') {
    if (parts.length === 2) return 'src-root';
    if (parts[1] === 'modules') {
      if (parts.length === 3) return 'modules-root';
      return 'modules-' + parts[2];
    }
    if (parts[1] === 'backends') return 'backends';
    if (parts[1] === 'bin') return 'bin';
    if (parts[1] === 'actors') return 'actors';
    if (parts[1] === 'llm_clients') return 'llm_clients';
    return 'src-root';
  }
  if (top === 'tests') {
    if (parts[1] === 'integration') return 'tests-integration';
    if (parts[1] === 'property') return 'tests-property';
    if (parts[1] === 'unit') return 'tests-unit';
    if (parts[1] === 'e2e_user_harness') return 'tests-e2e';
    return 'tests-root';
  }
  if (top === 'sdk') {
    if (parts[1] === 'typescript') return 'sdk-typescript';
    if (parts[1] === 'python') return 'sdk-python';
    if (parts[1] === 'continue') return 'sdk-continue';
    return 'sdk';
  }
  if (top === 'deploy' || top === 'k8s') return 'deploy';
  if (top === 'docs') {
    if (parts.length >= 3 && parts[1] === 'superpowers') return 'docs-superpowers';
    return 'docs';
  }
  if (top === 'openspec') return 'openspec';
  if (top === '.github') return 'ci-cd';
  if (top === 'schemas') return 'schemas';
  if (top === 'proto') return 'proto';
  if (top === 'examples') return 'examples';
  if (top === 'fixtures') return 'fixtures';
  if (top === 'vscode-extension') return 'vscode-extension';
  if (top === 'scripts') return 'scripts';
  if (top === 'huggingface-space') return 'deploy';
  return top;
}

// Build directory groups
const directoryGroups = {};
fileNodes.forEach(n => {
  const fp = getFilePath(n);
  const grp = getDirGroup(fp);
  if (!directoryGroups[grp]) directoryGroups[grp] = [];
  directoryGroups[grp].push(n.id);
});

// Node type groups
const nodeTypeGroups = {};
fileNodes.forEach(n => {
  if (!nodeTypeGroups[n.type]) nodeTypeGroups[n.type] = [];
  nodeTypeGroups[n.type].push(n.id);
});

// Build ID->group map
const idToGroup = {};
fileNodes.forEach(n => {
  const fp = getFilePath(n);
  idToGroup[n.id] = getDirGroup(fp);
});

// Import adjacency
const fanIn = {}, fanOut = {};
fileNodes.forEach(n => { fanIn[n.id] = 0; fanOut[n.id] = 0; });
importEdges.forEach(e => {
  if (fanOut[e.source] !== undefined) fanOut[e.source]++;
  if (fanIn[e.target] !== undefined) fanIn[e.target]++;
});

// Cross-category edges
const crossCatMap = {};
allEdges.forEach(e => {
  const srcNode = allNodes.find(n => n.id === e.source);
  const tgtNode = allNodes.find(n => n.id === e.target);
  if (!srcNode || !tgtNode) return;
  const key = `${srcNode.type}->${tgtNode.type}:${e.type}`;
  crossCatMap[key] = (crossCatMap[key] || 0) + 1;
});
const crossCategoryEdges = Object.entries(crossCatMap).map(([k, count]) => {
  const [types, edgeType] = k.split(':');
  const [fromType, toType] = types.split('->');
  return { fromType, toType, edgeType, count };
});

// Inter-group imports
const interGroupMap = {};
importEdges.forEach(e => {
  const fromGrp = idToGroup[e.source];
  const toGrp = idToGroup[e.target];
  if (!fromGrp || !toGrp || fromGrp === toGrp) return;
  const key = `${fromGrp}->${toGrp}`;
  interGroupMap[key] = (interGroupMap[key] || 0) + 1;
});
const interGroupImports = Object.entries(interGroupMap).map(([k, count]) => {
  const [from, to] = k.split('->');
  return { from, to, count };
}).sort((a, b) => b.count - a.count);

// Intra-group density
const intraGroupDensity = {};
Object.keys(directoryGroups).forEach(grp => {
  const members = new Set(directoryGroups[grp]);
  let internal = 0, total = 0;
  importEdges.forEach(e => {
    const srcInGrp = members.has(e.source);
    const tgtInGrp = members.has(e.target);
    if (srcInGrp || tgtInGrp) { total++; if (srcInGrp && tgtInGrp) internal++; }
  });
  intraGroupDensity[grp] = { internalEdges: internal, totalEdges: total, density: total ? internal/total : 0 };
});

// Pattern matching
const PATTERNS = {
  'bin': 'entry', 'actors': 'service', 'llm_clients': 'service',
  'src-root': 'service', 'modules-root': 'service',
  'modules-temporal_indexer': 'service', 'modules-symbolic_store': 'service',
  'modules-procedural_cache': 'service', 'modules-perception_adapter': 'service',
  'modules-aureus_bridge': 'service', 'modules-integration_layer': 'api',
  'modules-loop_engine': 'service', 'modules-mcp_bridge': 'api',
  'modules-openmanus_bridge': 'api', 'modules-world_model': 'service',
  'modules-world_model_enhanced': 'service', 'modules-coherence': 'service',
  'modules-self_model': 'service',
  'backends': 'data', 'schemas': 'types', 'proto': 'types',
  'fixtures': 'data', 'deploy': 'infrastructure', 'ci-cd': 'ci-cd',
  'docs': 'documentation', 'docs-superpowers': 'documentation',
  'openspec': 'documentation',
  'tests-integration': 'test', 'tests-property': 'test',
  'tests-unit': 'test', 'tests-e2e': 'test', 'tests-root': 'test',
  'sdk-typescript': 'sdk', 'sdk-python': 'sdk', 'sdk-continue': 'sdk',
  'sdk': 'sdk', 'examples': 'documentation',
  'vscode-extension': 'sdk', 'scripts': 'infrastructure',
  'root': 'config'
};
const patternMatches = {};
Object.keys(directoryGroups).forEach(grp => {
  patternMatches[grp] = PATTERNS[grp] || 'utility';
});

// Deployment topology
const allFilePaths = fileNodes.map(n => getFilePath(n));
const infraFiles = [];
let hasDockerfile = false, hasCompose = false, hasK8s = false, hasTerraform = false, hasCI = false;
fileNodes.forEach(n => {
  const fp = getFilePath(n);
  if (/Dockerfile/.test(fp)) { hasDockerfile = true; infraFiles.push(fp); }
  if (/docker-compose/.test(fp)) { hasCompose = true; infraFiles.push(fp); }
  if (/deploy\/helm/.test(fp) || /k8s/.test(fp)) { hasK8s = true; infraFiles.push(fp); }
  if (/\.tf$|\.tfvars$/.test(fp)) { hasTerraform = true; infraFiles.push(fp); }
  if (/\.github\/workflows|\.gitlab-ci|Jenkinsfile/.test(fp)) { hasCI = true; infraFiles.push(fp); }
});

// Data pipeline
const schemaFiles = fileNodes.filter(n => n.type === 'schema' || /\.proto$|\.graphql$|\.gql$/.test(getFilePath(n))).map(n => n.id);
const migrationFiles = fileNodes.filter(n => /migration/.test(getFilePath(n))).map(n => n.id);
const dataModelFiles = fileNodes.filter(n => /memory_record|memory_store|symbolic_store|temporal_indexer/.test(getFilePath(n))).map(n => n.id);
const apiHandlerFiles = fileNodes.filter(n => /web_server|mcp_server|grpc_server|integration_layer/.test(getFilePath(n))).map(n => n.id);

// Doc coverage
const grpsWithDocs = Object.entries(directoryGroups)
  .filter(([g]) => g === 'docs' || g === 'docs-superpowers' || g === 'openspec').map(([g]) => g);

// Dependency direction
const depDir = interGroupImports.filter(e => e.count > 0).map(e => ({
  dependent: e.from, dependsOn: e.to
}));

// File stats
const filesPerGroup = {};
Object.entries(directoryGroups).forEach(([g, ids]) => { filesPerGroup[g] = ids.length; });
const nodeTypeCounts = {};
Object.entries(nodeTypeGroups).forEach(([t, ids]) => { nodeTypeCounts[t] = ids.length; });

const result = {
  scriptCompleted: true,
  directoryGroups,
  nodeTypeGroups,
  crossCategoryEdges,
  interGroupImports,
  intraGroupDensity,
  patternMatches,
  deploymentTopology: { hasDockerfile, hasCompose, hasK8s, hasTerraform, hasCI, infraFiles },
  dataPipeline: { schemaFiles, migrationFiles, dataModelFiles, apiHandlerFiles },
  docCoverage: {
    groupsWithDocs: grpsWithDocs.length,
    totalGroups: Object.keys(directoryGroups).length,
    coverageRatio: grpsWithDocs.length / Object.keys(directoryGroups).length,
    undocumentedGroups: Object.keys(directoryGroups).filter(g => !grpsWithDocs.includes(g))
  },
  dependencyDirection: depDir,
  fileStats: {
    totalFileNodes: fileNodes.length,
    filesPerGroup,
    nodeTypeCounts
  },
  fileFanIn: fanIn,
  fileFanOut: fanOut
};

fs.writeFileSync(outputPath, JSON.stringify(result, null, 2));
console.error('Script completed successfully.');
process.exit(0);
