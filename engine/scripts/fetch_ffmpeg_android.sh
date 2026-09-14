#!/usr/bin/env bash
# 将 Android 各 ABI 的 ffmpeg 可执行文件复制到 vendor/ffmpeg/android-{abi}/ffmpeg
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENGINE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VENDOR_ROOT="${ENGINE_ROOT}/vendor/ffmpeg"
KHANG_NT_RELEASE="https://github.com/Khang-NT/ffmpeg-binary-android/releases/download/2018-07-31"
HZW_ARM64_URL="https://github.com/hzw1199/Android-FFmpeg-Prebuilt/raw/main/ffmpeg-8.1.1/bin/ffmpeg"

declare -a ABIS=("arm64-v8a" "armeabi-v7a" "x86" "x86_64")

khang_nt_asset() {
  case "$1" in
    arm64-v8a) echo "arm64-v8a-full.tar.bz2" ;;
    armeabi-v7a) echo "armv7-a-full.tar.bz2" ;;
    x86) echo "i686-full.tar.bz2" ;;
    x86_64) echo "x86_64-full.tar.bz2" ;;
    *)
      echo "不支持的 ABI: $1" >&2
      return 1
      ;;
  esac
}

copy_to_vendor() {
  local abi="$1"
  local src="$2"
  local dest_dir="${VENDOR_ROOT}/android-${abi}"
  local dest="${dest_dir}/ffmpeg"
  mkdir -p "${dest_dir}"
  cp -f "${src}" "${dest}"
  chmod +x "${dest}"
  echo "已安装 android-${abi} ffmpeg -> ${dest}"
}

download_khang_nt() {
  local abi="$1"
  local asset
  asset="$(khang_nt_asset "${abi}")"
  local tmpdir archive src_bin
  tmpdir="$(mktemp -d)"
  archive="${tmpdir}/${asset}"

  echo "从 Khang-NT 下载 ${asset}..."
  if ! curl -fsSL "${KHANG_NT_RELEASE}/${asset}" -o "${archive}"; then
    rm -rf "${tmpdir}"
    return 1
  fi
  if ! tar -xjf "${archive}" -C "${tmpdir}"; then
    rm -rf "${tmpdir}"
    return 1
  fi
  src_bin="$(find "${tmpdir}" -type f -name ffmpeg | head -n 1)"
  if [[ -z "${src_bin}" || ! -f "${src_bin}" ]]; then
    echo "在 ${asset} 中未找到 ffmpeg" >&2
    rm -rf "${tmpdir}"
    return 1
  fi
  copy_to_vendor "${abi}" "${src_bin}"
  rm -rf "${tmpdir}"
}

download_arm64_modern() {
  local tmpdir src_bin
  tmpdir="$(mktemp -d)"
  src_bin="${tmpdir}/ffmpeg"
  echo "从 Android-FFmpeg-Prebuilt 下载 arm64-v8a..."
  if ! curl -fsSL "${HZW_ARM64_URL}" -o "${src_bin}"; then
    rm -rf "${tmpdir}"
    return 1
  fi
  copy_to_vendor "arm64-v8a" "${src_bin}"
  rm -rf "${tmpdir}"
}

fetch_abi() {
  local abi="$1"
  local dest="${VENDOR_ROOT}/android-${abi}/ffmpeg"
  if [[ -x "${dest}" ]]; then
    echo "已存在 android-${abi} ffmpeg -> ${dest}"
    return 0
  fi

  if [[ "${abi}" == "arm64-v8a" ]]; then
    if download_arm64_modern; then
      return 0
    fi
  fi
  download_khang_nt "${abi}"
}

failed=0
for abi in "${ABIS[@]}"; do
  if ! fetch_abi "${abi}"; then
    echo "error: 获取 android-${abi} ffmpeg 失败" >&2
    failed=1
  fi
done

if [[ "${failed}" -ne 0 ]]; then
  exit 1
fi
