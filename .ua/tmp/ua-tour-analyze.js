#!/usr/bin/env node
'use strict';
const fs = require('fs');

const inputPath = process.argv[2];
const outputPath = process.argv[3];
if (!inputPath || !outputPath) { process.stderr.write('Usage: node ua-tour-analyze.js <input> <output>\n'); process.exit(1); }

const data = JSON.parse(fs.readFileSync(inputPath, 'utf8'));
const { nodes, edges, layers } = data;

// --- Build index ---
const nodeMap = {};
nodes.forEach(n => { nodeMap[n.id] = n; });

// --- A. Fan-In (how many nodes point TO this node) ---
const fanIn = {};
const fanOut = {};
nodes.forEach(n => { fanIn[n.id] = 0; fanOut[n.id] = 0; });
edges.forEach(e => {
  if (fanIn[e.target] !== undefined) fanIn[e.target]++;
  if (fanOut[e.source] !== undefined) fanOut[e.source]++;
});

const fanInRanking = Object.entries(fanIn)
  .sort((a,b) => b[1]-a[1]).slice(0,20)
  .map(([id,v]) => ({ id, fanIn: v, name: nodeMap[id]?.name||id }));

const fanOutRanking = Object.entries(fanOut)
  .sort((a,b) => b[1]-a[1]).slice(0,20)
  .map(([id,v]) => ({ id, fanOut: v, name: nodeMap[id]?.name||id }));

// --- C. Entry Point Candidates ---
const entryFilenames = new Set([
  'index.ts','index.js','main.ts','main.js','app.ts','app.js',
  'server.ts','server.js','mod.rs','main.go','main.py','main.rs',
  'manage.py','app.py','wsgi.py','asgi.py','run.py','__main__.py',
  'Application.java','Main.java','Program.cs','config.ru','index.php',
  'App.swift','Application.kt','main.cpp','main.c','lib.rs','cli.rs'
]);

const fanInValues = Object.values(fanIn);
const fanOutValues = Object.values(fanOut);
fanInValues.sort((a,b) => a-b);
fanOutValues.sort((a,b) => b-a);
const fanInP25 = fanInValues[Math.floor(fanInValues.length*0.25)];
const fanOutP90 = fanOutValues[Math.floor(fanOutValues.length*0.10)];

const scores = {};
nodes.forEach(n => {
  let score = 0;
  if (n.type === 'document') {
    const p = (n.filePath||n.id).replace(/^[^:]+:/,'');
    if (p === 'README.md') score += 5;
    else if (p.match(/^[^/]+\.md$/)) score += 2;
  } else if (n.type === 'file') {
    const parts = (n.filePath||n.name||'').replace(/^[^:]+:/,'').split('/');
    const fname = parts[parts.length-1];
    if (entryFilenames.has(fname)) score += 3;
    if (parts.length <= 2) score += 1;
    if (fanOut[n.id] >= fanOutP90) score += 1;
    if (fanIn[n.id] <= fanInP25) score += 1;
  }
  if (score > 0) scores[n.id] = score;
});

const entryPointCandidates = Object.entries(scores)
  .sort((a,b) => b[1]-a[1]).slice(0,5)
  .map(([id,score]) => ({ id, score, name: nodeMap[id]?.name||id, type: nodeMap[id]?.type, summary: nodeMap[id]?.summary||'' }));

// --- D. BFS from top code entry point ---
const topCode = entryPointCandidates.find(e => nodeMap[e.id]?.type === 'file');
const bfsStart = topCode?.id || entryPointCandidates[0]?.id;

const bfsOrder = [];
const depthMap = {};
if (bfsStart) {
  const queue = [[bfsStart, 0]];
  depthMap[bfsStart] = 0;
  const visited = new Set([bfsStart]);
  const fwdEdges = {};
  edges.forEach(e => {
    if (e.type === 'imports' || e.type === 'calls') {
      if (!fwdEdges[e.source]) fwdEdges[e.source] = [];
      fwdEdges[e.source].push(e.target);
    }
  });
  while (queue.length) {
    const [cur, d] = queue.shift();
    bfsOrder.push(cur);
    (fwdEdges[cur]||[]).forEach(next => {
      if (!visited.has(next)) {
        visited.add(next);
        depthMap[next] = d+1;
        queue.push([next, d+1]);
      }
    });
  }
}

const byDepth = {};
Object.entries(depthMap).forEach(([id,d]) => {
  if (!byDepth[d]) byDepth[d] = [];
  byDepth[d].push(id);
});

// --- E. Non-code files ---
const nonCodeFiles = { documentation: [], infrastructure: [], data: [], config: [] };
nodes.forEach(n => {
  const entry = { id: n.id, name: n.name, type: n.type, summary: (n.summary||'').slice(0,120) };
  if (n.type === 'document') nonCodeFiles.documentation.push(entry);
  else if (['service','pipeline','resource'].includes(n.type)) nonCodeFiles.infrastructure.push(entry);
  else if (['table','schema','endpoint'].includes(n.type)) nonCodeFiles.data.push(entry);
  else if (n.type === 'config') nonCodeFiles.config.push(entry);
});

// --- F. Tightly coupled clusters ---
const edgePairs = {};
edges.forEach(e => {
  const key = [e.source,e.target].sort().join('|||');
  if (!edgePairs[key]) edgePairs[key] = { a: e.source, b: e.target, count: 0 };
  edgePairs[key].count++;
});
// bidirectional = edge A->B and B->A both exist
const bidir = new Set();
edges.forEach(e => {
  const rev = edges.find(r => r.source === e.target && r.target === e.source);
  if (rev) bidir.add([e.source,e.target].sort().join('|||'));
});

// Build clusters: start from bidirectional pairs, expand
const clusterSeeds = [];
bidir.forEach(key => {
  const [a,b] = key.split('|||');
  clusterSeeds.push({ nodes: new Set([a,b]), edgeCount: 2 });
});

// Expand: add nodes connected to 2+ cluster members
clusterSeeds.forEach(cluster => {
  let changed = true;
  while (changed) {
    changed = false;
    nodes.forEach(n => {
      if (cluster.nodes.has(n.id)) return;
      let connCount = 0;
      cluster.nodes.forEach(cn => {
        if (edges.find(e => (e.source===n.id&&e.target===cn)||(e.source===cn&&e.target===n.id))) connCount++;
      });
      if (connCount >= 2) { cluster.nodes.add(n.id); cluster.edgeCount += connCount; changed = true; }
    });
  }
});

// Deduplicate and filter size 2-5
const clusterMap = new Map();
clusterSeeds.forEach(c => {
  const key = Array.from(c.nodes).sort().join(',');
  if (!clusterMap.has(key) && c.nodes.size >= 2 && c.nodes.size <= 5) {
    clusterMap.set(key, { nodes: Array.from(c.nodes), edgeCount: c.edgeCount });
  }
});
const clusters = Array.from(clusterMap.values())
  .sort((a,b) => b.edgeCount-a.edgeCount).slice(0,10);

// --- G. Layers ---
const layersOut = { count: layers.length, list: layers.map(l => ({ id:l.id, name:l.name, description:l.description||'' })) };

// --- H. Node summary index ---
const nodeSummaryIndex = {};
nodes.forEach(n => {
  nodeSummaryIndex[n.id] = { name: n.name, type: n.type, summary: (n.summary||'').slice(0,150) };
});

const result = {
  scriptCompleted: true,
  entryPointCandidates,
  fanInRanking,
  fanOutRanking,
  bfsTraversal: { startNode: bfsStart, order: bfsOrder, depthMap, byDepth },
  nonCodeFiles,
  clusters,
  layers: layersOut,
  nodeSummaryIndex,
  totalNodes: nodes.length,
  totalEdges: edges.length
};

fs.writeFileSync(outputPath, JSON.stringify(result, null, 2));
process.exit(0);
