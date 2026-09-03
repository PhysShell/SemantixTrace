#!/usr/bin/env bash
# E0 stability check: second identical run of ts/service (and later
# hipster/service once its stream is idle), compared against the first run.
# Waits for the harness to release the namespace checkout before starting.
set -u
NEZHA=${NEZHA:-/home/user/Nezha}
PY=${PY:-/home/user/.venv-nezha/bin/python}
RUNROOT=${RUNROOT:-/home/user/e0-runs}
SCRIPTDIR="$(cd "$(dirname "$0")" && pwd)"
NS=${1:?namespace}
LEVEL=${2:?level}
WAIT_MARK=${3:?harness.log line to wait for}

until grep -q "$WAIT_MARK" "$RUNROOT/harness.log"; do sleep 20; done

copy="$RUNROOT/checkout-$NS"
out="$RUNROOT/$NS-$LEVEL-run2"
PRISTINE="$RUNROOT/pristine-log_template"
mkdir -p "$out"
rm -rf "$copy/log_template"
cp -a "$PRISTINE" "$copy/log_template"
rm -f "$copy"/log/*_nezha.log

echo "[$(date -u +%FT%TZ)] START $NS/$LEVEL run2" >> "$RUNROOT/harness.log"
t0=$(date +%s)
( cd "$copy" && "$PY" ./main.py --ns "$NS" --level "$LEVEL" ) \
    > "$out/console.log" 2>&1
rc=$? ; t1=$(date +%s)
echo "exit_code=$rc wall_seconds=$((t1-t0))" > "$out/run-meta.txt"
echo "[$(date -u +%FT%TZ)] DONE $NS/$LEVEL run2 rc=$rc wall=$((t1-t0))s" >> "$RUNROOT/harness.log"
cp "$copy"/log/*_nezha.log "$out/artifact.log" 2>/dev/null
cp -a "$copy/log_template" "$out/log_template-after"
( cd "$out" && sha256sum log_template-after/* console.log artifact.log > sha256.txt 2>/dev/null )
"$PY" "$SCRIPTDIR/dump_templates.py" "$copy" "$NS" "$out/templates.json" \
    >> "$out/run-meta.txt" 2>&1
rm -rf "$copy/log_template"
cp -a "$PRISTINE" "$copy/log_template"
