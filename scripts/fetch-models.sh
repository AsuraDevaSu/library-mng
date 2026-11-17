#!/usr/bin/env bash
# Downloads the face models for the bag deposit desk from the OpenCV Model Zoo
# and verifies their checksums.
#   YuNet (face detection)   — MIT
#   SFace (face recognition) — Apache-2.0
set -euo pipefail
cd "$(dirname "$0")/../models"
BASE=https://github.com/opencv/opencv_zoo/raw/main/models
fetch() {
  local path=$1 sha=$2 file
  file=$(basename "$path")
  if ! echo "$sha  $file" | sha256sum -c --status 2>/dev/null; then
    echo "downloading $file"
    curl -fsSL -o "$file" "$BASE/$path"
  fi
  echo "$sha  $file" | sha256sum -c
}
fetch face_detection_yunet/face_detection_yunet_2023mar.onnx 8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4
fetch face_recognition_sface/face_recognition_sface_2021dec.onnx 0ba9fbfa01b5270c96627c4ef784da859931e02f04419c829e83484087c34e79
