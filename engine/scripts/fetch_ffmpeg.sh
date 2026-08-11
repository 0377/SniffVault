#!/usr/bin/env bash
# 将当前宿主平台的 ffmpeg 复制到 vendor/ffmpeg/{os}-{arch}/ffmpeg
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENGINE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VENDOR_ROOT="${ENGINE_ROOT}/vendor/ffmpeg"
BTBN_BASE="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest"

raw_os="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "${raw_os}" in
  darwin) os="macos" ;;
  linux) os="linux" ;;
  mingw* | msys* | cygwin*) os="windows" ;;
  *)
    echo "不支持的操作系统: ${raw_os}" >&2
    exit 1
    ;;
esac

raw_arch="$(uname -m)"
case "${raw_arch}" in
  x86_64 | amd64) arch="x86_64" ;;
  aarch64 | arm64) arch="aarch64" ;;
  *)
    echo "不支持的架构: ${raw_arch}" >&2
    exit 1
    ;;
esac

target_dir="${VENDOR_ROOT}/${os}-${arch}"
mkdir -p "${target_dir}"

if [[ "${os}" == "windows" ]]; then
  dest="${target_dir}/ffmpeg.exe"
  binary_name="ffmpeg.exe"
else
  dest="${target_dir}/ffmpeg"
  binary_name="ffmpeg"
fi

if [[ -x "${dest}" ]]; then
  echo "已存在 ffmpeg -> ${dest}"
  "${dest}" -version | head -n 1
  exit 0
fi

# GHA macOS 上 PATH 过长会导致 brew link / opt 符号链接异常。
if [[ -n "${GITHUB_ACTIONS:-}" ]] && [[ "${os}" == "macos" ]]; then
  export PATH="/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/opt/homebrew/sbin"
fi

find_cellar_ffmpeg() {
  local cellar_root candidate
  for cellar_root in /opt/homebrew/Cellar/ffmpeg /usr/local/Cellar/ffmpeg; do
    if [[ ! -d "${cellar_root}" ]]; then
      continue
    fi
    candidate="$(find "${cellar_root}" -type f -name "${binary_name}" -path '*/bin/*' 2>/dev/null | head -n 1)"
    if [[ -n "${candidate}" && -f "${candidate}" ]]; then
      echo "${candidate}"
      return 0
    fi
  done
  return 1
}

copy_to_vendor() {
  local src="$1"
  if [[ ! -f "${src}" ]]; then
    echo "源文件不存在: ${src}" >&2
    return 1
  fi
  cp -f "${src}" "${dest}"
  chmod +x "${dest}"
}

resolve_ffmpeg() {
  if [[ "${os}" == "macos" ]]; then
    local cellar_bin
    if cellar_bin="$(find_cellar_ffmpeg)"; then
      echo "${cellar_bin}"
      return 0
    fi
  fi
  if command -v ffmpeg >/dev/null 2>&1; then
    local cmd resolved
    cmd="$(command -v ffmpeg)"
  if [[ -f "${cmd}" ]]; then
      resolved="$(cd "$(dirname "${cmd}")" && pwd -P)/$(basename "${cmd}")"
      if [[ -f "${resolved}" ]]; then
        echo "${resolved}"
        return 0
      fi
    fi
  fi
  return 1
}

install_with_brew() {
  if [[ "${os}" != "macos" ]] || ! command -v brew >/dev/null 2>&1; then
    return 1
  fi
  echo "通过 Homebrew 安装 ffmpeg..."
  brew install ffmpeg 2>&1 || true

  local cellar_bin
  if cellar_bin="$(find_cellar_ffmpeg)"; then
    copy_to_vendor "${cellar_bin}"
    return 0
  fi
  return 1
}

download_macos_npm() {
  if [[ "${os}" != "macos" ]]; then
    return 1
  fi

  local npm_pkg version="4.1.5"
  case "${arch}" in
    x86_64) npm_pkg="darwin-x64" ;;
    aarch64) npm_pkg="darwin-arm64" ;;
    *) return 1 ;;
  esac

  local tmpdir src_bin
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "${tmpdir}"' RETURN

  echo "从 @ffmpeg-installer/${npm_pkg} 下载..."
  curl -fsSL \
    "https://registry.npmjs.org/@ffmpeg-installer/${npm_pkg}/-/${npm_pkg}-${version}.tgz" \
    -o "${tmpdir}/pkg.tgz"
  tar -xzf "${tmpdir}/pkg.tgz" -C "${tmpdir}"
  src_bin="${tmpdir}/package/ffmpeg"
  if [[ ! -f "${src_bin}" ]]; then
    echo "在 npm 包中未找到 ffmpeg" >&2
    return 1
  fi
  copy_to_vendor "${src_bin}"
}

extract_tar_xz() {
  local archive="$1" dest_dir="$2"
  if tar -xJf "${archive}" -C "${dest_dir}" 2>/dev/null; then
    return 0
  fi
  if command -v python3 >/dev/null 2>&1; then
    python3 - "${archive}" "${dest_dir}" <<'PY'
import lzma
import sys
import tarfile

archive, dest_dir = sys.argv[1], sys.argv[2]
with lzma.open(archive) as xz_stream:
    with tarfile.open(fileobj=xz_stream) as tar:
        tar.extractall(dest_dir)
PY
    return 0
  fi
  echo "需要 xz 或 python3 以解压 ${archive}" >&2
  return 1
}

download_btbn() {
  local asset archive_name tmpdir extract_root src_bin
  case "${os}-${arch}" in
    linux-x86_64)
      asset="ffmpeg-master-latest-linux64-gpl.tar.xz"
      archive_name="ffmpeg.tar.xz"
      ;;
    linux-aarch64)
      asset="ffmpeg-master-latest-linuxarm64-gpl.tar.xz"
      archive_name="ffmpeg.tar.xz"
      ;;
    windows-x86_64)
      asset="ffmpeg-master-latest-win64-gpl.zip"
      archive_name="ffmpeg.zip"
      ;;
    windows-aarch64)
      asset="ffmpeg-master-latest-winarm64-gpl.zip"
      archive_name="ffmpeg.zip"
      ;;
    *)
      return 1
      ;;
  esac

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "${tmpdir}"' RETURN

  echo "从 BtbN 下载 ${asset}..."
  curl -fsSL "${BTBN_BASE}/${asset}" -o "${tmpdir}/${archive_name}"

  case "${archive_name}" in
    *.tar.xz)
      extract_tar_xz "${tmpdir}/${archive_name}" "${tmpdir}"
      ;;
    *.zip)
      if command -v unzip >/dev/null 2>&1; then
        unzip -q "${tmpdir}/${archive_name}" -d "${tmpdir}"
      elif command -v powershell.exe >/dev/null 2>&1; then
        powershell.exe -NoProfile -Command \
          "Expand-Archive -Path '${tmpdir}/${archive_name}' -DestinationPath '${tmpdir}' -Force"
      else
        echo "未找到 unzip 或 powershell，无法解压 ${archive_name}" >&2
        return 1
      fi
      ;;
  esac

  extract_root="$(find "${tmpdir}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
  src_bin="${extract_root}/bin/${binary_name}"
  if [[ ! -f "${src_bin}" ]]; then
    src_bin="$(find "${tmpdir}" -type f -name "${binary_name}" | head -n 1)"
  fi
  if [[ -z "${src_bin}" || ! -f "${src_bin}" ]]; then
    echo "在 ${asset} 中未找到 ${binary_name}" >&2
    return 1
  fi

  copy_to_vendor "${src_bin}"
}

install_macos_ffmpeg() {
  if [[ -n "${GITHUB_ACTIONS:-}" ]] && download_macos_npm; then
    return 0
  fi
  install_with_brew
}

if [[ "${os}" == "macos" ]]; then
  if src="$(resolve_ffmpeg)"; then
    copy_to_vendor "${src}"
  elif install_macos_ffmpeg; then
    :
  elif download_macos_npm; then
    :
  else
    cat >&2 <<EOF
未找到 ffmpeg，且自动安装失败。

macOS: brew install ffmpeg && $0
目标路径: ${dest}
EOF
    exit 1
  fi
elif src="$(resolve_ffmpeg)"; then
  copy_to_vendor "${src}"
elif download_btbn; then
  :
else
  cat >&2 <<EOF
未找到 ffmpeg，且自动安装失败。

Linux/Windows: 请检查网络后重试，或手动从 BtbN 下载并加入 PATH:
  https://github.com/BtbN/FFmpeg-Builds/releases

目标路径: ${dest}
EOF
  exit 1
fi

echo "已安装 ffmpeg -> ${dest}"
"${dest}" -version | head -n 1
