#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [ "$#" -ne 1 ]; then
  echo "usage: $0 backups/ull-top-2000-YYYY-MM-DD-HHMMSS.sql.gz"
  exit 1
fi

if [ ! -f "$1" ]; then
  echo "backup not found: $1"
  exit 1
fi

echo "This will overwrite the compose database with $1."
echo "Press Ctrl+C to cancel; continuing in 3 seconds."
sleep 3

gunzip -c "$1" | docker compose exec -T db sh -c \
  'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 --single-transaction'

echo "Restore complete."
