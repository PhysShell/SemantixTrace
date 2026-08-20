# E0 environment manifest — Nezha historical reproduction

Status: NEZHA-HISTORICAL runs executed with this environment. Every deviation
from the artifact's declared environment is listed in "Deviations" and echoed
in `docs/experiments/nezha-semantixtrace/decisions.log.md`.

## Artifact under reproduction

- Repository: fork `PhysShell/Nezha` of `IntelligentDDS/Nezha`
- Commit (pinned): `d8140101fdb4e7dfb60d3ef9f64706f382b68470`
  ("Merge pull request #15 from zhjiang22/main", 2025-05-20)
- Working tree: clean at run time (verified by the harness,
  `experiment/nezha/scripts/run_e0.sh`)
- Dataset: shipped inside the repository; SHA256 manifest of all 1238
  data/state files: `experiment/nezha/manifests/dataset-manifest.sha256`

## Host

- Linux 6.18.5 x86_64, 4 CPUs, 15 GiB RAM (Claude Code remote container)
- Python: 3.11.15 (`python3 -m venv`)

## Python packages (installed, exact)

| package | artifact requirements.txt | installed | deviation? |
|---|---|---|---|
| drain3 | 0.9.10 | **0.9.10** | no (exact pin honored) |
| more_itertools | 8.12.0 | **8.12.0** | no |
| psutil | 5.9.0 | **5.9.0** | no |
| PyYAML | 6.0.1 | **6.0.1** | no |
| numpy | 1.15.4 | **1.26.4** | YES — 1.15.4 does not build on Python 3.11 |
| pandas | 0.23.4 | **1.5.3** | YES — 0.23.4 does not build on Python 3.11 |
| matplotlib | 3.3.4 | 3.8.4 | YES (import-only dependency of alarm.py) |
| tqdm | (absent from requirements.txt) | 4.66.4 | artifact defect: `pattern_ranker.py` imports tqdm but requirements.txt omits it |
| jsonpickle | (transitive, drain3) | 1.5.1 | drain3 0.9.10's resolved dependency |
| cachetools | (transitive, drain3) | 4.2.1 | drain3 0.9.10's resolved dependency |

## Deviations and their risk assessment

1. **Python 3.11.15 instead of "Python 3.6 recommended".** The artifact's
   README recommends 3.6 but states "any python3 version should be fine".
   The pinned commit's HEAD merge (PR #15) is itself a fix *for* post-3.6
   pandas behavior, i.e. upstream intends the code to run on modern Python.
2. **numpy 1.26.4 / pandas 1.5.3 instead of 1.15.4 / 0.23.4.** The pinned
   versions are Python≤3.7-era C extensions and fail to build on 3.11
   (attempt recorded; `pip` metadata-generation error). The operations used
   by the active code path (`read_csv`, `.loc`/`.iloc` label and positional
   indexing, `np.ceil`, `np.mean`, `np.std`, `np.percentile`) have stable
   semantics across these versions. Residual risk is acknowledged and bounded
   by the outcome check: reproduction is compared case-by-case against the
   authors' committed result logs (`log/*.log` in the artifact).
3. **drain3 pinned exactly (0.9.10)** because template mining is
   algorithm-relevant state: the shipped `log_template/*.bin` state files are
   loaded and *mutated* by every run; a different drain3 version could assign
   different cluster IDs. The harness restores pristine `.bin` state from git
   before every configuration run.

## Reproduction commands

```
python3 -m venv ~/.venv-nezha
~/.venv-nezha/bin/pip install drain3==0.9.10 more_itertools==8.12.0 \
    psutil==5.9.0 PyYAML==6.0.1 numpy==1.26.4 pandas==1.5.3 \
    matplotlib==3.8.4 tqdm==4.66.4
bash experiment/nezha/scripts/run_e0.sh      # all four headline configs
```
