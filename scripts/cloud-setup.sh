#!/usr/bin/env bash
set -euo pipefail

fail() { printf 'cloud setup: %s\n' "$*" >&2; exit 1; }
[[ $(uname -s) == Linux ]] || fail 'Linux only; provision macOS prerequisites manually (see AGENTS.md).'
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture="$repo/.ci-seed-me"
remote=https://github.com/jon-devlapaz/seed-me.git
ref=$(awk '$1 == "repository:" && $2 == "jon-devlapaz/seed-me" { found=1; next }
  found && $1 == "ref:" { print $2; exit }' "$repo/.github/workflows/ci.yml")
[[ $ref =~ ^[0-9a-f]{40}$ ]] || fail 'CI must specify one full Seed Me commit SHA.'

# Reject unrelated or dirty checkouts before fetching packages or changing tools.
if [[ -e $fixture ]]; then
  [[ -d $fixture/.git ]] || fail '.ci-seed-me exists but is not a Git checkout; move it aside yourself.'
  [[ $(git -C "$fixture" remote get-url origin) == "$remote" ]] || fail '.ci-seed-me has an unexpected origin; leaving it untouched.'
  status=$(git -C "$fixture" status --porcelain) || fail 'Cannot read .ci-seed-me status; leaving it untouched.'
  [[ -z $status ]] || fail '.ci-seed-me is dirty; leaving it untouched.'
fi

missing=false
for cmd in curl python3 cc make ar tar xz sha256sum; do
  command -v "$cmd" >/dev/null || missing=true
done
if $missing; then
  command -v apt-get >/dev/null || fail 'Install curl, Python 3, a C toolchain, tar, xz and coreutils; automatic packages need Debian/Ubuntu.'
  apt=(apt-get)
  if [[ $(id -u) != 0 ]]; then
    command -v sudo >/dev/null && sudo -n true || fail 'Missing prerequisites need root or non-interactive sudo.'
    apt=(sudo -n apt-get)
  fi
  "${apt[@]}" update
  "${apt[@]}" install -y --no-install-recommends ca-certificates curl build-essential python3 xz-utils
fi

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$repo/.cloud-tools/node/bin:$PATH"
tools="$repo/.cloud-tools"
mkdir -p "$tools"
if ! command -v rustup >/dev/null; then
  curl -fsSL --proto '=https' --tlsv1.2 https://sh.rustup.rs -o "$tools/rustup-init.sh"
  sh "$tools/rustup-init.sh" -y --profile minimal --default-toolchain stable --no-modify-path
fi
rustup toolchain install stable --profile minimal --component clippy --component rustfmt

if ! command -v node >/dev/null; then
  case $(uname -m) in
    x86_64) arch=x64;;
    aarch64|arm64) arch=arm64;;
    *) fail 'Automatic Node install supports Linux x86_64 and arm64 only.';;
  esac
  dist=https://nodejs.org/dist/latest-v24.x
  curl -fsSL --proto '=https' --tlsv1.2 "$dist/SHASUMS256.txt" -o "$tools/SHASUMS256.txt"
  archive=$(awk -v suffix="-linux-$arch.tar.xz" 'index($2, suffix) && substr($2, length($2)-length(suffix)+1) == suffix { print $2; exit }' "$tools/SHASUMS256.txt")
  [[ $archive =~ ^node-v24\.[0-9]+\.[0-9]+-linux-(x64|arm64)\.tar\.xz$ ]] || fail 'No matching Node 24 package checksum.'
  curl -fsSL --proto '=https' --tlsv1.2 "$dist/$archive" -o "$tools/$archive"
  (cd "$tools"; awk -v name="$archive" '$2 == name { print }' SHASUMS256.txt | sha256sum --check --strict -)
  mkdir -p "$tools/node"
  tar -xJf "$tools/$archive" --strip-components=1 -C "$tools/node"
fi
node --version
python3 --version
rustc +stable --version
cargo +stable clippy --version

if [[ ! -d $fixture/.git ]]; then
  git init -q "$fixture"
  git -C "$fixture" remote add origin "$remote"
fi
git -C "$fixture" fetch --depth=1 origin "$ref"
git -C "$fixture" checkout --detach --quiet FETCH_HEAD
[[ $(git -C "$fixture" rev-parse HEAD) == "$ref" ]] || fail 'Seed Me checkout did not match the CI pin.'
[[ -f $fixture/skills/seed-me/SKILL.md ]] || fail 'Pinned fixture has no Seed Me skill.'
# Official Python helpers create bytecode caches; ignore only those generated files.
python3 - "$fixture/.git/info/exclude" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
if '__pycache__/' not in path.read_text().splitlines():
    with path.open('a') as file:
        file.write('\n__pycache__/\n')
PY

printf '\nSetup complete. Set these in your current shell, then run the AGENTS.md verification block:\n'
printf 'export PATH=%q:"$PATH"\n' "${CARGO_HOME:-$HOME/.cargo}/bin:$tools/node/bin"
printf 'export SEED_ME_TEST_SKILL=%q\n' "$fixture/skills/seed-me/SKILL.md"
printf 'export TINKERY_NODE=/tinkery-TEST-no-node\n'
