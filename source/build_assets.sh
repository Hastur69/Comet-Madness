#!/usr/bin/env bash
# Re-renders every audio asset from source using tools/soundforge, then encodes
# the music to OGG Vorbis and copies everything into assets/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/build_audio"
( cd "$ROOT/tools/soundforge" && cargo run --release -- "$OUT" )
mkdir -p "$ROOT/assets/music" "$ROOT/assets/sfx"
for f in "$OUT"/music/*.wav; do
  ffmpeg -hide_banner -loglevel error -y -i "$f" -c:a libvorbis -q:a 6 "$ROOT/assets/music/$(basename "${f%.wav}").ogg"
done
cp "$OUT"/sfx/*.wav "$ROOT/assets/sfx/"
echo "Audio assets rebuilt."
