# Upstream Issue-Tracker Audit: IntelligentDDS/Nezha

Audit date: 2026-08-20
Method: WebFetch of public GitHub HTML pages (issue list, per-issue pages, PR list, PR pages).
Scope: ALL issues (#1–#14; #7 and #15 are pull requests, not issues) plus both merged PRs.
Pinned commit of interest: `d8140101fdb4e7dfb60d3ef9f64706f382b68470`.

NOTE ON METHOD: Claims below are recorded as stated by their authors. No claim has been
verified against the Nezha source code in this audit (that is a separate workstream).
Comment counts were cross-checked against the comment-count badges on the issue list page:
**every single issue in this tracker has 0 comments** — the maintainer (yuxiaoba) has never
replied in a comment on any issue. Closed issues were closed without any recorded explanation.

---

## Issue inventory

| # | Title | State |
|---|-------|-------|
| 1 | Abnormal reproduction results | Closed |
| 2 | About Nezhe, beseek help | Closed |
| 3 | Reproducibility matter | Closed |
| 4 | How did you come up with the current Drain config? | Closed |
| 5 | MicroRCA and TraceAnomaly implementations | Closed |
| 6 | Replication package | Open |
| 7 | *(PR)* Timestamp and time did not match. | Merged 2024-06-05 |
| 8 | Wrong time stamp in metrics file | Open |
| 9 | baseline code on this dataset | Open |
| 10 | about the function get_deepth_pod | Open |
| 11 | abnormal_pattern_ranker in pattern_ranker.py | Open |
| 12 | topk computation error in evaluation fkt of pattern_ranker.py | Open |
| 13 | Is it possible to open source the code for the demo in paper | Closed |
| 14 | Missing Logs and Traces in construct_data？ | Open |
| 15 | *(PR)* fix bugs after python3.6 in alarm.py | Merged 2025-05-20 |

---

## Issue #1 — "Abnormal reproduction results"

- Author: lyr5333
- Date: September 5, 2023
- State: Closed
- Body: **"No description provided."** (the issue has an empty body — either filed empty or
  content later removed; the title alleges abnormal reproduction results)
- Comments: none (0).
- Note: Issue #3 references this issue, saying other users obtained "the exact same results",
  implying #1 originally contained (or was understood to represent) a concrete failed
  reproduction on the Hipster/OnlineBoutique dataset.
- Maintainer response: none. No comment from yuxiaoba or anyone else.
- Resolution state: closed (no stated reason, no fix referenced).

## Issue #2 — "About Nezhe, beseek help"

- Author: molujia
- Date: November 13, 2023
- State: Closed
- Claims (paraphrase of body; author plans to use Nezha as a baseline across five
  multimodal datasets):
  1. **Dependency conflict**: requirements.txt pins `Orange3_Associate==1.1.9` and
     `numpy==1.15.4`, but Orange3_Associate 1.1.9 requires `Orange3>=3.25.0` and
     `openTSNE>=0.6.1`, which in turn require `numpy>=1.16.0` — an unsatisfiable
     dependency chain as specified.
  2. **Poor reproduction results**: after running Nezha with the specified versions on
     Python 3.6, obtained results were: "AS@1: 5.357143%, AS@3: 7.142857%, AS@5: 7.142857%"
     — described as far below the paper's numbers.
  3. **Question on trace filtering**: asks why the codebase filters traces, using only
     traces whose IDs appear in the trace_id file, as implemented in
     `data_integrate.py`'s `data_integrate()` function.
- Comments: none (0).
- Maintainer response: none.
- Resolution state: closed (no stated reason, no fix referenced).

## Issue #3 — "Reproducibility matter"

- Author: phamquiluan
- Date: November 15, 2023
- State: Closed
- Claims (body, addressed to @yuxiaoba): tried to reproduce Nezha results on "Hister shop"
  [Hipster shop] following the installation instructions carefully but was
  **"unable to reproduce the results"**. Attached a screenshot of the obtained results
  (screenshot content not retrievable from the HTML page) and noted that
  **others experienced identical numerical outcomes, referencing issue #1**. Asked whether
  the maintainer had seen this problem before and requested help.
- Comments: none (0).
- Maintainer response: none.
- Resolution state: closed (no stated reason, no fix referenced).

## Issue #4 — "How did you come up with the current Drain config?"

- Author: phamquiluan
- Date: November 22, 2023
- State: Closed
- Body (verbatim): "Hi my idol @yuxiaoba 😄 / I came across the Drain config and really
  interested in it. How did you obtain these numbers and regular expressions?
  https://github.com/IntelligentDDS/Nezha/blob/main/log_template/drain3_hipster.ini /
  Thank you so much in advance ^^"
- Referenced file: `log_template/drain3_hipster.ini`
- Comments: none (0).
- Maintainer response: none.
- Resolution state: closed (question never answered on the tracker).

## Issue #5 — "MicroRCA and TraceAnomaly implementations"

- Author: MaxiStefan
- Date: December 6, 2023
- State: Closed
- Body (verbatim as captured): "I have gone through your paper and published code here on
  GitHub and I was wondering if you've made public the way you implemented MicroRCA and
  TraceAnomaly for the comparison with Nezha." Author notes they could not locate these
  baseline implementations in the codebase despite thorough inspection.
- Comments: none (0).
- Maintainer response: none.
- Resolution state: closed (baseline implementations were never published on the tracker;
  see also open issues #6 and #9 repeating the request).

## Issue #6 — "Replication package"

- Author: MaxiStefan
- Date: December 7, 2023
- State: **Open**
- Body (verbatim as captured): "Hi, I was wondering if you have made or can make public the
  complete replication package of your study. I was particularly interested in your
  implementation of MicroRCA and TraceAnomaly algorithms for your benchmark. Thanks in
  advance!" (references the paper PDF at
  https://yuxiaoba.github.io/publication/nezha23/nezha23.pdf)
- Comments: none (0).
- Maintainer response: none.
- Resolution state: open, unanswered.

## Issue #8 — "Wrong time stamp in metrics file"

- Author: mmantyla (same author as merged PR #7)
- Date: December 14, 2023
- State: **Open**
- Claim (verbatim): "This file has incorrect timestamps. They all start with 18 when they
  should start with 16, e.g. 1861140279 when it should be 1661140279."
- File referenced: `construct_data/2022-08-22/metric/adservice-5f6585d649-fnmft_metric.csv`
- Additional claim: the author performed a "raw string replacement" while building a data
  loader for Nezha into LogLead, and did **not** verify all files — i.e., other metric
  files in the dataset may carry the same corrupted-timestamp defect.
- Comments: none (0). PR #7 is NOT linked from this issue (PR #7 fixed a different,
  fault_list.json timestamp mismatch — see PR section).
- Maintainer response: none.
- Resolution state: open; the metric-CSV timestamp corruption has no upstream fix
  (no PR touches the metric CSVs).

## Issue #9 — "baseline code on this dataset"

- Author: daixixiwang
- Date: August 6, 2024
- State: **Open**
- Body (verbatim as captured): "Dear Author, I am interested in the baseline methods you
  used in your paper on this dataset. Would it be possible for you to share the code
  implementation of the baseline method? It would be greatly appreciated if you could make
  it available for everyone to learn from and improve upon classic algorithms. Thank you
  for your consideration. Best regards,"
- Comments: none (0).
- Maintainer response: none.
- Resolution state: open, unanswered (third unanswered request for baselines, after #5, #6).

## Issue #10 — "about the function get_deepth_pod" [SPECIAL ATTENTION]

- Author: Shenyyyyyyyy
- Date: November 22, 2024
- State: **Open**
- Claim: the function `get_deepth_pod` in `data_integrate.py` contains "many levels of
  loops" and appears to **enter an infinite loop under certain conditions**. The author asks
  what the function is meant to compute and under exactly which scenarios the infinite loop
  is triggered. (This is the function that walks the adjacency list upward from a target
  event; the reproduction-study concern is that it cycles over collapsed event IDs.)
- Code quoted in the issue body (verbatim):

```python
def get_deepth_pod(self, traget_event):
    pod = ""
    deepth = 0
    while True:
        flag = False
        for key in self.adjacency_list.keys():
            for item in self.adjacency_list[key]:
                if traget_event == item.event:
                    traget_event = key.event
                    if deepth == 0:
                        pod = item.pod
                    flag = True
                    if "start" in key.event and "TraceID" not in key.event:
                        deepth = deepth + 1
                    break
            if flag == True:
                return deepth, pod
        if flag == False:
            break
    return deepth, pod
```

- File referenced: `data_integrate.py`
- Comments: none (0).
- Maintainer response: none.
- Resolution state: open; no fix, no acknowledgement.

## Issue #11 — "abnormal_pattern_ranker in pattern_ranker.py" [SPECIAL ATTENTION]

- Author: virsel
- Date: December 15, 2024
- State: **Open**
- Claim: in `abnormal_pattern_ranker` (file `pattern_ranker.py`), the line

```python
score_dict = sorted(score_dict, reverse=True)
```

  sorts the dictionary **by its keys (event-ID strings) instead of by the score values**.
  The issue includes an example `score_dict` of ~49 entries mapping event-ID-pair keys to
  float scores (roughly 0.50–1.0), e.g. entries like `'77_78': 1.0` and `'106_21': 0.518...`.
  The captured current output ordering begins `['84_85', '84_104', '77_88', '77_86' ...]` —
  i.e., lexicographic/numeric ordering of keys — whereas the expected ordering would put
  the highest-scoring pattern first (score 1.0, then 0.579..., then 0.528..., ...).
- Expected behavior per the reporter: sort by the corresponding score values in descending
  order (e.g., `sorted(score_dict, key=score_dict.get, reverse=True)` or similar).
- File referenced: `pattern_ranker.py`
- Comments: none (0).
- Maintainer response: none.
- Resolution state: open; no fix, no acknowledgement.

## Issue #12 — "topk computation error in evaluation fkt of pattern_ranker.py" [SPECIAL ATTENTION]

- Author: virsel (same author as #11, filed the same day)
- Date: December 15, 2024
- State: **Open**
- Claim: in the evaluation function of `pattern_ranker.py`, when a result-list candidate
  contains a resource key (added because of a resource alert), **the `topk` rank counter is
  not incremented if the true root cause does not match that resource-key candidate** —
  the increment lives only in the `else` branch that handles non-resource candidates, so
  resource candidates that are wrong are skipped without advancing the rank, undercounting
  the rank of the true root cause (inflating top-k accuracy).
- Problematic code as quoted in the issue (verbatim as captured):

```python
if len(root_cause) == 1:
    for i in range(len(result_list)):
        if "resource" in result_list[i].keys():
            if str(root_cause[0]) in str(result_list[i]["resource"]) and str(fault["inject_pod"]) in str(result_list[i]["pod"]):
                top_list.append(topk)
                logger.info("%s Inject Ground Truth: %s, %s score %s", fault["inject_time"],
                            fault["inject_pod"], fault["inject_type"], topk)
                break
```

  ("when a candidate contains resource key...the topk variable dont gets incremented if
  the true root cause dont matches.")
- Proposed fix as quoted in the issue (verbatim as captured):

```python
if len(root_cause) == 1:
    for i in range(len(result_list)):
        if "resource_alert" in result_list[i].keys():
            if str(root_cause[0]) in str(result_list[i]["resource_alert"]) and inject_pod in str(result_list[i]["pod"]):
                break
        else:
            if i > 0:
                if result_list[i-1]["score"] == result_list[i]["score"] and result_list[i-1]["deepth"] == result_list[i]["deepth"]:
                    continue
        topk += 1
```

  i.e., move the `topk += 1` outside the conditional nesting so the rank advances on every
  non-matching candidate (with tie-handling on equal score+depth).
- File referenced: `pattern_ranker.py` (evaluation function)
- Comments: none (0).
- Maintainer response: none.
- Resolution state: open; no fix, no acknowledgement. Directly bears on evaluation
  correctness (reported top-k accuracy numbers).

## Issue #13 — "Is it possible to open source the code for the demo in paper"

- Author: dinghanfei
- Date: March 29, 2025
- State: Closed
- Body (verbatim as captured): "I noticed that your team mentioned implementing a demo in
  the paper to demonstrate root causes candidates for SREs. I wonder if you could open
  source the corresponding source code. Thanks in advance!" (an attached image is no longer
  accessible — expired token URL)
- Comments: none (0).
- Maintainer response: none.
- Resolution state: closed without answer.

## Issue #14 — "Missing Logs and Traces in construct_data？" [SPECIAL ATTENTION]

- Author: YixiangTang
- Date: April 11, 2025
- State: **Open**
- Body (verbatim as captured): "Hi author, the log and metric data in construct_data seems
  to be incomplete, only a csv file." The reporter points to the directories
  `construct_data/2022-08-22/log` and `construct_data/2022-08-22/trace` as containing only
  a single CSV file each rather than the expected complete log/trace collections.
- Comments: none (0).
- Maintainer response: none.
- Resolution state: open; dataset-completeness claim unaddressed.

---

## Pull requests

### PR #7 — "Timestamp and time did not match." (MERGED)

- Author: mmantyla (also author of open issue #8)
- Opened: December 11, 2023; Merged: June 5, 2024 by yuxiaoba
- Merge commit: `45bcf45eea23add84faad1fcb772ea073540aff0`
- File changed: `rca_data/2022-08-22/2022-08-22-fault_list.json` (2 additions, 2 deletions)
- Change: corrects a ground-truth fault-list mismatch — `inject_time` updated from
  `"2022-08-22 03:53:10"` to `"2022-08-22 03:53:54"` to agree with the adjacent
  `inject_timestamp` value `"1661140434"` (a frontend-pod injection entry).
- Author's comment (verbatim): "Looked into to the data (cpu data) to resolve which is
  correct and it seems the timestamp is correct." (i.e., the Unix timestamp was taken as
  authoritative and the human-readable time corrected to match)
- Relative to pinned commit `d8140101`: **landed BEFORE** the pinned commit — this
  ground-truth fix IS included in the pinned tree.

### PR #15 — "fix bugs after python3.6 in alarm.py" (MERGED)

- Author: zhjiang22
- Opened & merged: May 20, 2025, merged by yuxiaoba
- Merge commit: `d8140101fdb4e7dfb60d3ef9f64706f382b68470` — **this merge IS the pinned
  commit** (i.e., PR #15 is the HEAD of the pinned tree).
- Claim/description (verbatim as captured): "After python 3.6, the index column will not be
  presented in the pandas.Dataframe. This can lead to the following code encountering an
  error: pod_spans = pod_reader.loc[[pod_name], ['SpanID', 'ParentID', 'EndTimeUnixNano']]"
- File changed: `alarm.py` (per title; files tab not itemized in fetch).
- Relative to pinned commit: **included** (it is the pinned commit itself).

### Before/after the pinned commit `d8140101fdb4e7dfb60d3ef9f64706f382b68470`

- Fixes landed BEFORE (or at) the pin, i.e. INCLUDED in the pinned tree:
  - PR #7 fault_list.json inject_time correction (merged 2024-06-05).
  - PR #15 alarm.py pandas-indexing fix (merge = pinned commit, 2025-05-20).
- Fixes landed AFTER the pin: **none**. There are no merged PRs after d8140101; every
  code-level defect reported in issues #8, #10, #11, #12 and the dataset gap in #14
  remains UNFIXED upstream as of the audit date.

---

## Cross-cutting observations

1. **Zero maintainer engagement on the tracker.** Every issue (#1–#14 excl. PRs) has a
   comment count of 0. The maintainer's only recorded activity is merging PR #7 and PR #15
   and closing issues #1–#5 and #13 without comment.
2. **Reproducibility failures reported independently three times** (#1, #2, #3), with #2
   giving concrete numbers (AS@1 5.36%, AS@3/AS@5 7.14%) and #3 stating other users hit
   identical numbers. All three were closed with no explanation or fix.
3. **Evaluation-correctness defects (#11 sorting, #12 top-k) and the potential infinite
   loop (#10)** are alleged with concrete code excerpts and remain open and unacknowledged;
   none is fixed in the pinned commit.
4. **Dataset-integrity defects**: corrupted metric timestamps (#8, "18…" vs "16…" prefix,
   possibly in more files than the one named) remain unfixed; the ground-truth fault-list
   time mismatch was fixed by PR #7; #14 alleges construct_data log/trace directories are
   incomplete (single CSV each).
5. **Baselines never released** despite three requests (#5 closed unanswered; #6 and #9
   still open): MicroRCA and TraceAnomaly comparison implementations are not public, nor
   is a full replication package or the paper's SRE demo (#13).
