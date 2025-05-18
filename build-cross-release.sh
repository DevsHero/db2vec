#!/usr/bin/env bash
set -euo pipefail

# 1) Name of your Rust binary (as in Cargo.toml)
BIN_NAME="db2vec"

# 2) Ensure cross is installed
if ! command -v cross &>/dev/null; then
  echo "❌ 'cross' not found – installing..."
  cargo install cross --git https://github.com/cross-rs/cross :contentReference[oaicite:0]{index=0}
fi

# 3) Output directory
DIST_DIR="$(pwd)/dist"
mkdir -p "$DIST_DIR"

# 4) List of targets
TARGETS=(
  "x86_64-unknown-linux-gnu"      # Linux x86_64 :contentReference[oaicite:1]{index=1}
  "aarch64-unknown-linux-gnu"     # Linux ARM64 :contentReference[oaicite:2]{index=2}
  "x86_64-pc-windows-gnu"         # Windows x64 :contentReference[oaicite:3]{index=3}
)

# 5) Build loop
for TARGET in "${TARGETS[@]}"; do
  echo "⏳ Building for $TARGET..."
  cross rustc --target "$TARGET" --release 
done

# 6) Copy binaries into dist/
echo "📂 Collecting binaries into $DIST_DIR..."
for TARGET in "${TARGETS[@]}"; do
  BIN_PATH="target/${TARGET}/release/${BIN_NAME}"
  # On Windows targets, add .exe
  if [[ "$TARGET" == *"windows"* ]]; then
    BIN_PATH+=".exe"
  fi

  if [[ -f "$BIN_PATH" ]]; then
    OUT_NAME="${BIN_NAME}-${TARGET}"
    # Preserve extension on Windows
    if [[ "$TARGET" == *"windows"* ]]; then
      OUT_NAME+=".exe"
    fi

    cp "$BIN_PATH" "$DIST_DIR/$OUT_NAME"
    echo "✅ $OUT_NAME"
  else
    echo "⚠️  Missing: $BIN_PATH"
  fi
done

echo "🎉 All done! Binaries are in $DIST_DIR."
