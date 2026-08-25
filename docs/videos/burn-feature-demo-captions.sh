#!/usr/bin/env bash
# Burn docs/videos/rgctl-feature-demo.srt onto the no-captions dashboard recording.
# Leaves rgctl-feature-demo-no-captions.mp4 untouched for comparison.
#
# Usage (from repo root):
#   ./docs/videos/burn-feature-demo-captions.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VID="$ROOT/docs/videos"
SRC="$VID/rgctl-feature-demo-no-captions.mp4"
SRT="$VID/rgctl-feature-demo.srt"
OUT="$VID/rgctl-feature-demo.mp4"

if [[ ! -f "$SRC" ]]; then
  echo "error: missing $SRC — record first:" >&2
  echo "  DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/record-feature-demo.mjs" >&2
  exit 1
fi
if [[ ! -f "$SRT" ]]; then
  echo "error: missing $SRT (recorder writes it)" >&2
  exit 1
fi

FFMPEG=ffmpeg
if [[ -x /opt/homebrew/opt/ffmpeg-full/bin/ffmpeg ]]; then
  FFMPEG=/opt/homebrew/opt/ffmpeg-full/bin/ffmpeg
elif ! ffmpeg -filters 2>/dev/null | grep -q 'subtitles'; then
  echo "error: ffmpeg lacks subtitles filter — brew install ffmpeg-full" >&2
  exit 1
fi

cd "$VID"
"$FFMPEG" -y -i rgctl-feature-demo-no-captions.mp4 \
  -vf "subtitles=filename=rgctl-feature-demo.srt:force_style='FontName=Menlo,FontSize=20,PrimaryColour=&H00FFFFFF&,OutlineColour=&H80000000&,BackColour=&H80000000&,BorderStyle=3,Outline=1,Shadow=0,MarginV=40,Alignment=2'" \
  -c:v libx264 -pix_fmt yuv420p -crf 20 \
  rgctl-feature-demo.mp4

echo "==> wrote $OUT"
ls -lh "$OUT" "$SRC"
