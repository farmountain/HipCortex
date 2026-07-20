# Plan: PR + release v0.5.7 with topo server bins + VSIX

1. Enhance release.yml: always artifact bins; package-vsix job stages multi-OS server/ + vsce
2. Push feat/optional-deep-wire + open PR → main
3. Tag v0.5.7 on branch tip; publish GitHub release (triggers binary CI with topo routes)
4. Upload local hipcortex-memory-0.5.7.vsix to release immediately
