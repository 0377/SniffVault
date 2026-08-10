#!/usr/bin/env bash
# 生成 HLS 测试 fixture：明文 TS 分片、AES-128 加密分片与 key.bin
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENGINE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FIXTURES="${ENGINE_ROOT}/tests/fixtures/hls"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "${os}" in darwin) os="macos" ;; esac
arch="$(uname -m)"
case "${arch}" in arm64) arch="aarch64" ;; esac
FFMPEG="${ENGINE_ROOT}/vendor/ffmpeg/${os}-${arch}/ffmpeg"
if [[ ! -x "${FFMPEG}" ]]; then
  if command -v ffmpeg >/dev/null 2>&1; then
    FFMPEG="$(command -v ffmpeg)"
  else
    echo "未找到 ffmpeg，请先运行 scripts/fetch_ffmpeg.sh" >&2
    exit 1
  fi
fi

KEY_HEX="00112233445566778899aabbccddeeff"
KEY_FILE="${WORK}/key.bin"
echo -n "${KEY_HEX}" | xxd -r -p > "${KEY_FILE}"

SRC_MP4="${WORK}/src.mp4"
"${FFMPEG}" -y -hide_banner -loglevel error \
  -f lavfi -i "testsrc=duration=4:size=320x240:rate=30" \
  -f lavfi -i "sine=frequency=440:duration=4" \
  -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest \
  "${SRC_MP4}"

write_media_playlist() {
  local dest="$1"
  local prefix="$2"
  local encrypted="$3"
  local seg_dir="$4"
  local seg_count="$5"

  {
    echo "#EXTM3U"
    echo "#EXT-X-VERSION:3"
    echo "#EXT-X-TARGETDURATION:10"
    echo "#EXT-X-MEDIA-SEQUENCE:0"
    if [[ "${encrypted}" == "1" ]]; then
      echo "#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\""
    fi
    for i in $(seq 0 $((seg_count - 1))); do
      echo "#EXTINF:1.5,"
      echo "${prefix}${seg_dir}/seg${i}.ts"
    done
    echo "#EXT-X-ENDLIST"
  } > "${dest}"
}

PLAIN_DIR="${WORK}/plain"
mkdir -p "${PLAIN_DIR}"
"${FFMPEG}" -y -hide_banner -loglevel error \
  -i "${SRC_MP4}" -c copy -f hls \
  -hls_time 1.5 -hls_playlist_type vod \
  -hls_segment_filename "${PLAIN_DIR}/seg%d.ts" \
  "${PLAIN_DIR}/media.m3u8"

ENC_DIR="${WORK}/encrypted"
mkdir -p "${ENC_DIR}"
KEY_INFO="${WORK}/key_info.txt"
cat > "${KEY_INFO}" <<EOF
${KEY_FILE}
${KEY_FILE}
EOF
"${FFMPEG}" -y -hide_banner -loglevel error \
  -i "${SRC_MP4}" -c copy -f hls \
  -hls_time 1.5 -hls_playlist_type vod \
  -hls_key_info_file "${KEY_INFO}" \
  -hls_segment_filename "${ENC_DIR}/seg%d.ts" \
  "${ENC_DIR}/media.m3u8"

mkdir -p "${FIXTURES}/segments" "${FIXTURES}/encrypted"
cp -f "${KEY_FILE}" "${FIXTURES}/key.bin"
rm -f "${FIXTURES}/segments"/*.ts "${FIXTURES}/encrypted"/*.ts
cp -f "${PLAIN_DIR}"/seg*.ts "${FIXTURES}/segments/"
cp -f "${ENC_DIR}"/seg*.ts "${FIXTURES}/encrypted/"

SEG_COUNT=$(ls -1 "${FIXTURES}/segments"/seg*.ts | wc -l | tr -d ' ')
write_media_playlist "${FIXTURES}/media.m3u8" "" 0 "segments" "${SEG_COUNT}"
write_media_playlist "${FIXTURES}/1080p.m3u8" "" 0 "segments" "${SEG_COUNT}"
write_media_playlist "${FIXTURES}/720p.m3u8" "" 0 "segments" 1
write_media_playlist "${FIXTURES}/encrypted.m3u8" "" 1 "encrypted" "${SEG_COUNT}"

echo "已生成 ${SEG_COUNT} 个明文分片与加密 fixture 到 ${FIXTURES}"
