#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="$root_dir/tests/fixtures/audio"
mkdir -p "$output_dir"

common=(-hide_banner -loglevel error -f lavfi -i "sine=frequency=440:sample_rate=16000:duration=0.25" -ac 1 -map_metadata -1 -bitexact)
ffmpeg -y "${common[@]}" -c:a pcm_s16le "$output_dir/tone.wav"
ffmpeg -y "${common[@]}" -c:a libmp3lame -b:a 64k "$output_dir/tone.mp3"
ffmpeg -y "${common[@]}" -c:a aac -b:a 64k -f adts "$output_dir/tone.aac"
ffmpeg -y "${common[@]}" -c:a aac -b:a 64k -movflags +faststart "$output_dir/tone.m4a"
ffmpeg -y "${common[@]}" -c:a flac "$output_dir/tone.flac"
ffmpeg -y "${common[@]}" -c:a libvorbis -q:a 4 "$output_dir/tone.ogg"

sha256sum "$output_dir"/tone.*
