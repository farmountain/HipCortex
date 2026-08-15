from pathlib import Path

pats = {
    "service_hipcortex": b'"service":"hipcortex"',
    "status_ok": b'"status":"ok"',
    "version_key": b'"version"',
    "plain_ok_route": b'async { "ok" }',
    "application_json": b"application/json",
    "pkg_0_4_9": b"0.4.9",
    "pkg_0_5_0": b"0.5.0",
}

bins = [
    ("local_darwin_arm64", Path(r"D:\all_projects\HipCortex\vscode-extension\server\darwin\hipcortex-macos-arm64")),
    ("release_darwin_arm64", Path(r"D:\all_projects\HipCortex\.tmp-hc-data\hipcortex-macos-arm64")),
    ("local_win", Path(r"D:\all_projects\HipCortex\vscode-extension\server\win32\hipcortex-windows-amd64.exe")),
]

for label, path in bins:
    data = path.read_bytes()
    hits = {k: (data.find(v) >= 0) for k, v in pats.items()}
    print(label, "size", len(data), hits)
