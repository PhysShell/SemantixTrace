#!/usr/bin/env bash
# Post-harness E0 processing: independent evaluation of all four configs
# plus generated comparison tables.
set -u
RUNROOT=${RUNROOT:-/home/user/e0-runs}
NEZHA=${NEZHA:-/home/user/Nezha}
PY=${PY:-/home/user/.venv-nezha/bin/python}
SCRIPTDIR="$(cd "$(dirname "$0")" && pwd)"
EVALDIR="$RUNROOT/eval"
mkdir -p "$EVALDIR"

for config in hipster-service hipster-inner ts-service ts-inner; do
    ns=${config%%-*}
    if [ -f "$RUNROOT/$config/artifact.log" ]; then
        "$PY" "$SCRIPTDIR/../evaluators/independent_eval.py" \
            --artifact-log "$RUNROOT/$config/artifact.log" \
            --templates "$RUNROOT/$config/templates.json" \
            --nezha-dir "$NEZHA" --ns "$ns" \
            --out "$EVALDIR/$config.eval.json" \
            | tee "$EVALDIR/$config.eval.txt"
    else
        echo "SKIP $config: no artifact.log yet"
    fi
done

"$PY" "$SCRIPTDIR/make_e0_report.py" "$RUNROOT" "$NEZHA" \
    > "$EVALDIR/e0-tables.md"
echo "tables: $EVALDIR/e0-tables.md"
