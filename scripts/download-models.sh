#!/usr/bin/env bash
# 从 GitHub Releases 下载 whisper ggml 模型到 ./models
set -euo pipefail

REPO="linuxjoy-sudo/smart-BC"
TAG="whisper-models-v1"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"
DEST="$(dirname "$0")/../models"

mkdir -p "$DEST"

for name in ggml-base.bin ggml-small.bin; do
  if [ ! -f "${DEST}/${name}" ]; then
    echo "Downloading ${name} ..."
    curl -fL --retry 3 -o "${DEST}/${name}" "${BASE_URL}/${name}"
  else
    echo "${name} already present"
  fi
done

echo "Models ready:"
ls -lh "$DEST"
