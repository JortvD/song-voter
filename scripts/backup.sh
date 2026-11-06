#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
umask 077

mkdir -p backups
timestamp="$(date +%Y-%m-%d-%H%M%S)"
output="backups/playlist-voter-${timestamp}.sql.gz"
temporary="${output}.tmp"
trap 'rm -f "$temporary"' EXIT

docker compose exec -T db sh -c \
  'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists --no-owner' \
  | gzip > "$temporary"

# A partial gzip stream must never look like a usable backup.
gzip -t "$temporary"
test -s "$temporary"
mv "$temporary" "$output"
trap - EXIT

echo "Backup written to $output"
find backups -type f -name 'playlist-voter-*.sql.gz' -mtime +14 -delete
