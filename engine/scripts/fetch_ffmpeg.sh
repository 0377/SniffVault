#!/usr/bin/env bash
# 将当前宿主平台的 ffmpeg 复制到 vendor/ffmpeg/{os}-{arch}/ffmpeg
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENGINE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VENDOR_ROOT="${ENGINE_ROOT}/vendor/ffmpeg"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "${arch}" in
  x86_64) arch="x86_64" ;;
  aarch64 | arm64) arch="aarch64" ;;
  *)
    echo "不支持的架构: ${arch}" >&2
    exit 1
    ;;
esac

target_dir="${VENDOR_ROOT}/${os}-${arch}"
mkdir -p "${target_dir}"

if [[ "${os}" == "darwin" || "${os}" == "linux" ]]; then
  dest="${target_dir}/ffmpeg"
else
  dest="${target_dir}/ffmpeg.exe"
fi

resolve_ffmpeg() {
  if command -v ffmpeg >/dev/null 2>&1; then
    command -v ffmpeg
    return 0
  fi
  if [[ "${os}" == "darwin" ]] && command -v brew >/dev/null 2>&1; then
    local prefix
    prefix="$(brew --prefix ffmpeg 2>/dev/null || true)"
    if [[ -n "${prefix}" && -x "${prefix}/bin/ffmpeg" ]]; then
      echo "${prefix}/bin/ffmpeg"
      return 0
    fi
  fi
  return 1
}

if ! src="$(resolve_ffmpeg)"; then
  cat >&2 <<EOF
未找到 ffmpeg。

macOS: brew install ffmpeg && $0
Linux: 安装系统 ffmpeg，或从 BtbN 下载后解压并加入 PATH:
  https://github.com/BtbN/FFmpeg-Builds/releases

目标路径: ${dest}
EOF
  exit 1
fi

cp -f "${src}" "${dest}"
chmod +x "${dest}"

echo "已安装 ffmpeg -> ${dest}"
"${dest}" -version | head -n 1
