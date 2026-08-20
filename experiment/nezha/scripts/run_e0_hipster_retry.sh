#!/usr/bin/env bash
# Sequential retry of the hipster configurations after the OOM failure of
# the concurrent 64-worker run (see decisions.log D-004). Worker count is
# bounded via NEZHA_MAX_WORKERS (repro-infra commit ae34750 in the fork);
# everything else is identical to run_e0.sh. Runs service, inner, and a
# second service run (determinism check) strictly one after another.
set -u
NEZHA=${NEZHA:-/home/user/Nezha}
PY=${PY:-/home/user/.venv-nezha/bin/python}
RUNROOT=${RUNROOT:-/home/user/e0-runs}
SCRIPTDIR="$(cd "$(dirname "$0")" && pwd)"
export NEZHA_MAX_WORKERS=${NEZHA_MAX_WORKERS:-8}

PRISTINE="$RUNROOT/pristine-log_template"
copy="$RUNROOT/checkout-hipster"
rm -rf "$copy"
cp -al "$NEZHA" "$copy"
rm -rf "$copy/.git" "$copy/log_template" "$copy/log"
cp -a "$PRISTINE" "$copy/log_template"
mkdir -p "$copy/log"

run_config() { # level outdirname
    local level=$1 out="$RUNROOT/$2"
    mkdir -p "$out"
    rm -rf "$copy/log_template"
    cp -a "$PRISTINE" "$copy/log_template"
    rm -f "$copy"/log/*_nezha.log
    echo "[$(date -u +%FT%TZ)] START hipster/$level -> $2 (workers=$NEZHA_MAX_WORKERS)" >> "$RUNROOT/harness.log"
    local t0=$(date +%s)
    ( cd "$copy" && "$PY" ./main.py --ns hipster --level "$level" ) \
        > "$out/console.log" 2>&1
    local rc=$? t1=$(date +%s)
    echo "exit_code=$rc wall_seconds=$((t1-t0)) workers=$NEZHA_MAX_WORKERS" > "$out/run-meta.txt"
    echo "[$(date -u +%FT%TZ)] DONE hipster/$level -> $2 rc=$rc wall=$((t1-t0))s" >> "$RUNROOT/harness.log"
    cp "$copy"/log/*_nezha.log "$out/artifact.log" 2>/dev/null
    cp -a "$copy/log_template" "$out/log_template-after"
    ( cd "$out" && sha256sum log_template-after/* console.log artifact.log > sha256.txt 2>/dev/null )
    "$PY" "$SCRIPTDIR/dump_templates.py" "$copy" hipster "$out/templates.json" \
        >> "$out/run-meta.txt" 2>&1
}

run_config service hipster-service
run_config inner hipster-inner
run_config service hipster-service-run2
echo "[$(date -u +%FT%TZ)] HIPSTER RETRY ALL DONE" >> "$RUNROOT/harness.log"
