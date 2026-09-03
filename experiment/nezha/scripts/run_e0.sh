#!/usr/bin/env bash
# E0 historical reproduction harness for the Nezha artifact.
#
# Runs the four headline configurations of the pinned Nezha commit
# (d8140101fdb4e7dfb60d3ef9f64706f382b68470) WITHOUT algorithm modifications
# and captures every output needed for independent evaluation:
#   - full console stream (stdout+stderr, includes DEBUG "Soted Result List")
#   - the artifact's own dated log file from ./log/
#   - the run-mutated drain3 template state (.bin) + a JSON dump of id->template
#   - wall-clock timing and exit codes
#
# Isolation: each namespace gets a hardlink copy of the checkout with REAL
# copies of the mutable paths (log_template/, log/), because Nezha's template
# miner persists learned state into log_template/<ns>.bin during a run and
# both --level runs of one namespace must start from the pristine committed
# state. Runs inside one namespace are sequential; the two namespaces run in
# parallel (disjoint .bin files).
set -u

NEZHA=${NEZHA:-/home/user/Nezha}
PY=${PY:-/home/user/.venv-nezha/bin/python}
RUNROOT=${RUNROOT:-/home/user/e0-runs}
SCRIPTDIR="$(cd "$(dirname "$0")" && pwd)"
STAMP=$(date -u +%Y%m%dT%H%M%SZ)

mkdir -p "$RUNROOT"
echo "pinned commit: $(git -C "$NEZHA" rev-parse HEAD)" | tee "$RUNROOT/meta-$STAMP.txt"
git -C "$NEZHA" status --short >> "$RUNROOT/meta-$STAMP.txt"

# Pristine template snapshot (verify checkout is clean there first).
if ! git -C "$NEZHA" diff --quiet -- log_template; then
    echo "FATAL: $NEZHA/log_template is dirty; refusing to snapshot" >&2
    exit 1
fi
PRISTINE="$RUNROOT/pristine-log_template"
rm -rf "$PRISTINE"
cp -a "$NEZHA/log_template" "$PRISTINE"

make_copy() { # ns
    local ns=$1 copy="$RUNROOT/checkout-$ns"
    rm -rf "$copy"
    cp -al "$NEZHA" "$copy"           # hardlink copy (cheap, shares data files)
    rm -rf "$copy/.git" "$copy/log_template" "$copy/log" "$copy/.venv-nezha"
    cp -a "$PRISTINE" "$copy/log_template"   # real copy: mutated during runs
    mkdir -p "$copy/log"
}

run_config() { # ns level
    local ns=$1 level=$2 copy="$RUNROOT/checkout-$ns"
    local out="$RUNROOT/$ns-$level"
    mkdir -p "$out"
    # pristine template state per config
    rm -rf "$copy/log_template"
    cp -a "$PRISTINE" "$copy/log_template"
    rm -f "$copy"/log/*_nezha.log

    echo "[$(date -u +%FT%TZ)] START $ns/$level" | tee -a "$RUNROOT/harness.log"
    local t0=$(date +%s)
    ( cd "$copy" && "$PY" ./main.py --ns "$ns" --level "$level" ) \
        > "$out/console.log" 2>&1
    local rc=$? t1=$(date +%s)
    echo "exit_code=$rc wall_seconds=$((t1-t0))" > "$out/run-meta.txt"
    echo "[$(date -u +%FT%TZ)] DONE $ns/$level rc=$rc wall=$((t1-t0))s" | tee -a "$RUNROOT/harness.log"

    cp "$copy"/log/*_nezha.log "$out/artifact.log" 2>/dev/null
    cp -a "$copy/log_template" "$out/log_template-after"
    ( cd "$out" && sha256sum log_template-after/* console.log artifact.log > sha256.txt 2>/dev/null )
    "$PY" "$SCRIPTDIR/dump_templates.py" "$copy" "$ns" "$out/templates.json" \
        >> "$out/run-meta.txt" 2>&1
    # restore pristine state for the next config in this namespace
    rm -rf "$copy/log_template"
    cp -a "$PRISTINE" "$copy/log_template"
}

stream() { # ns
    local ns=$1
    make_copy "$ns"
    run_config "$ns" service
    run_config "$ns" inner
}

stream hipster &
HP=$!
stream ts &
TP=$!
wait $HP; wait $TP
echo "[$(date -u +%FT%TZ)] ALL DONE" | tee -a "$RUNROOT/harness.log"
