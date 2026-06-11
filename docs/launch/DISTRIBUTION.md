# Distribution Checklist

## 1. Show HN (Hacker News)

**Post:** `docs/launch/show-hn.md` — reviewed, includes intelligence layer
**Submit:** https://news.ycombinator.com/submit
**Title:** "Show HN: HipCortex — Rust AI memory with metacognitive intelligence layer"
**URL:** https://github.com/farmountain/HipCortex
**Timing:** Monday or Tuesday 9am ET

```bash
# Also cross-post to Reddit
# Post docs/launch/reddit-localllama.md to r/LocalLLaMA
```

## 2. npm (TypeScript SDK)

```bash
cd D:/all_projects/HipCortex/sdk/typescript
npm login
npm install
npm run build
npm test
npm publish
```

Package: `hipcortex` v0.2.0 — https://www.npmjs.com/package/hipcortex

## 3. VS Code Marketplace

```bash
cd D:/all_projects/HipCortex/vscode-extension
npm install
npm run compile
vsce package        # creates .vsix
vsce publish        # publishes to marketplace
```

Get PAT from: https://dev.azure.com → Personal Access Tokens
Publisher: `hipcortex`
Extension: `hipcortex-memory` v0.2.0

## 4. HuggingFace Space

```bash
cd D:/all_projects/HipCortex
huggingface-cli login
huggingface-cli repo create hipcortex --type space --sdk docker
git clone https://huggingface.co/spaces/<user>/hipcortex
cp huggingface-space/* hipcortex-space/
cd hipcortex-space
git add . && git commit -m "Initial deploy" && git push
```

Config: `huggingface-space/README.md` + `huggingface-space/Dockerfile`

## 5. GitHub Copilot Extension

**Plan:** `docs/launch/copilot-extension-plan.md`
**Effort:** ~3 days implementation
**Target:** 3M+ Copilot users

## 6. PyPI (already done)

```bash
pip install hipcortex  # v0.2.0 already published
```

## Post-Launch Checklist

- [ ] Monitor HN/Reddit comments for 48 hours
- [ ] Respond to GitHub issues within 24 hours
- [ ] Track npm/PyPI download counts
- [ ] Track VS Code extension installs
- [ ] Record demo video for HuggingFace Space
- [ ] Write case study: "Copilot billing cut 6× with HipCortex"
