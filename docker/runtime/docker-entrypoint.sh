#!/bin/sh
set -eu

if [ -z "${NYRO_ADMIN_TOKEN:-}" ]; then
  echo "NYRO_ADMIN_TOKEN is required when exposing the admin server on 0.0.0.0" >&2
  exit 1
fi

exec /app/nyro-server \
  --proxy-host 0.0.0.0 \
  --proxy-port 19530 \
  --admin-host 0.0.0.0 \
  --admin-port 19531 \
  --admin-token "${NYRO_ADMIN_TOKEN}" \
  --data-dir /var/lib/nyro \
  "$@"
