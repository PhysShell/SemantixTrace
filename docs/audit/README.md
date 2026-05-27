# audit/

Append-only audits that record reconciliations between past commits /
docs / data and current canonical state. Same purpose as `griff`'s
`audit/` directory: when something is wrong and rewriting history would
hurt more than honesty, the audit notes the divergence and the
canonical path forward.

No audits yet; this directory exists so the first audit can land
without restructuring the docs tree.
