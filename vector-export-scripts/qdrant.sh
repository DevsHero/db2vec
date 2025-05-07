#!/usr/bin/env bash
set -eo pipefail

QDRANT_URL="http://localhost:6333"

# 1. Discover all collections
collections=$(curl -s "${QDRANT_URL}/collections" \
               -H "Content-Type: application/json" \
             | jq -r '.result.collections[].name')

for col in $collections; do
  echo "Exporting collection: $col"

  # 2. Create snapshot (synchronous)
  resp=$(curl -s -X POST \
           "${QDRANT_URL}/collections/${col}/snapshots" \
           -H "Content-Type: application/json")
  snap=$(jq -r '.result.name' <<<"$resp")
  echo " Snapshot created: $snap"

  # 3. Download
  curl -s "${QDRANT_URL}/collections/${col}/snapshots/${snap}" \
       --output "${col}.snapshot"
  echo " Saved ${col}.snapshot"
done

echo "✅ All snapshots exported."
