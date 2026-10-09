#!/usr/bin/env bash
set -euo pipefail
task_repo="$(cd "$(dirname "$0")/.." && pwd)"
task_stage="$(mktemp -d /tmp/dev-lang-tcc-XXXXXX)"
task_revision=43c7708b85681a2fd4451c8a541af4494a8919b2
mkdir "$task_stage/src"
git -C "$task_stage/src" init -q
git -C "$task_stage/src" remote add origin https://repo.or.cz/tinycc.git
git -C "$task_stage/src" fetch --depth 1 origin "$task_revision"
git -C "$task_stage/src" checkout --detach FETCH_HEAD
cd "$task_stage/src"
./configure --prefix="$task_stage/install"
make -j8
make install
mkdir -p "$task_repo/dist/compiler/linux-x86_64/backend"
cp -r "$task_stage/install/." "$task_repo/dist/compiler/linux-x86_64/backend/"
cp COPYING "$task_repo/dist/compiler/linux-x86_64/backend/LICENSE"
git archive --format=tar.gz HEAD > "$task_repo/dist/compiler/linux-x86_64/backend/tinycc-source.tar.gz"
printf '%s\n' "$task_revision" > "$task_repo/dist/compiler/linux-x86_64/backend/REVISION"
"$task_repo/dist/compiler/linux-x86_64/backend/bin/tcc" -B"$task_repo/dist/compiler/linux-x86_64/backend/lib/tcc" -v
printf 'TinyCC source: %s\n' "$task_stage/src"
