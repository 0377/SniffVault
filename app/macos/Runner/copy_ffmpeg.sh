#!/bin/bash
set -euo pipefail

ENGINE_ROOT="${SRCROOT}/../../engine"
ARCH="$(uname -m)"
case "${ARCH}" in
  arm64) RUST_ARCH=aarch64 ;;
  x86_64) RUST_ARCH=x86_64 ;;
  *)
    echo "error: unsupported macOS arch: ${ARCH}" >&2
    exit 1
    ;;
esac

SRC="${ENGINE_ROOT}/vendor/ffmpeg/macos-${RUST_ARCH}/ffmpeg"
APP_RESOURCES="${BUILT_PRODUCTS_DIR}/${WRAPPER_NAME}/Contents/Resources"
DST="${APP_RESOURCES}/ffmpeg"

if [[ ! -f "${SRC}" ]]; then
  echo "error: ffmpeg not found at ${SRC}" >&2
  echo "Run: (cd engine && ./scripts/fetch_ffmpeg.sh)" >&2
  exit 1
fi

mkdir -p "${APP_RESOURCES}"
cp -f "${SRC}" "${DST}"
chmod +x "${DST}"
xattr -cr "${DST}" 2>/dev/null || true

if [[ -n "${EXPANDED_CODE_SIGN_IDENTITY:-}" ]] && [[ "${EXPANDED_CODE_SIGN_IDENTITY}" != "-" ]]; then
  codesign --force --sign "${EXPANDED_CODE_SIGN_IDENTITY}" --timestamp=none "${DST}"
else
  codesign --force --sign - "${DST}"
fi

echo "Bundled ffmpeg -> ${DST}"
