#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/cline-sub.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/cline-sub" ;;
    *) rel="x86_64-apple-darwin/cline-sub" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/cline-sub" ;;
    *) rel="x86_64-unknown-linux-musl/cline-sub" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/cline-sub.exe" "$here/target/release/cline-sub" "$here/target/debug/cline-sub.exe" "$here/target/debug/cline-sub"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "cline-sub binary not found in $here" >&2
exit 1
