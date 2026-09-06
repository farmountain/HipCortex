#!/bin/sh
set -e
# Railway volume mounts are often root-owned; make DATA_DIR writable for hipcortex.
DATA_DIR="${DATA_DIR:-/app/data}"
mkdir -p "$DATA_DIR"
if [ "$(id -u)" = "0" ]; then
  chown -R hipcortex:hipcortex "$DATA_DIR" || true
  exec runuser -u hipcortex -- webserver "$@"
fi
exec webserver "$@"
