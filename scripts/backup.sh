#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
umask 077

mkdir -p backups
timestamp="$(date +%Y-%m-%d-%H%M%S)"
output="backups/ull-top-2000-${timestamp}.sql.gz"

docker compose exec -T db sh -c \
  'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists --no-owner' \
  | gzip > "$output"

echo "Backup written to $output"
find backups -type f -name 'ull-top-2000-*.sql.gz' -mtime +14 -delete
