#!/usr/bin/env python3
"""Dump the drain3 template state of a Nezha checkout to JSON.

Consumes the (possibly run-mutated) log_template/<ns>.bin via the same
drain3==0.9.10 code path Nezha itself uses, so the id->template mapping is
exactly what pattern_ranker's from_id_to_template would have returned.

Usage: dump_templates.py <nezha_checkout_dir> <ns> <out_json>
"""
import json
import sys


def main() -> None:
    checkout, ns, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
    sys.path.insert(0, checkout)
    from drain3 import TemplateMiner
    from drain3.file_persistence import FilePersistence
    from drain3.template_miner_config import TemplateMinerConfig

    config = TemplateMinerConfig()
    config.load(checkout + "/log_template/drain3_" + ns + ".ini")
    config.profiling_enabled = False
    persistence = FilePersistence(checkout + "/log_template/" + ns + ".bin")
    miner = TemplateMiner(persistence, config=config)

    mapping = {}
    for cluster in miner.drain.clusters:
        mapping[cluster.cluster_id] = {
            "template": cluster.get_template(),
            "size": cluster.size,
        }
    with open(out_path, "w") as f:
        json.dump(
            {"ns": ns, "cluster_count": len(mapping), "clusters": mapping},
            f,
            indent=1,
            sort_keys=True,
        )
    print("dumped %d clusters to %s" % (len(mapping), out_path))


if __name__ == "__main__":
    main()
