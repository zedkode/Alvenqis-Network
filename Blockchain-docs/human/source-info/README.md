# Alvenqis Source Information

Status: Canonical documentation set

This directory summarizes Alvenqis identity, accepted technical direction,
architecture, maturity, and unresolved decisions. It must be read together with
the current implementation and the decision registers.

## Precedence

1. Current consensus implementation and validation tests define behavior.
2. `../../internal/memory/DECISIONS.md` records explicitly accepted decisions in the private workspace.
3. `../../internal/memory/OPEN_QUESTIONS.md` records unresolved decisions in the private workspace.
4. This source-info set explains those facts for humans and downstream tools.
5. Recommendations and historical audits do not override accepted decisions.

See `../DOCUMENTATION_POLICY.md` for the workspace-wide documentation rules.

## Canonical read order

1. `ALVENQIS_00_CITESTE_PRIMUL_DECIZII_DESCHISE.md` — read-first decisions and gaps
2. `ALVENQIS_01_SOURCE_INFO_MASTER.md` — identity and fixed launch facts
3. `ALVENQIS_02_ARCHITECTURE_AND_PRODUCT_LAYERS.md` — current and planned layers
4. `ALVENQIS_03_ROADMAP_AND_STRUCTURE.md` — maturity-gated execution order
5. `ALVENQIS_04_AGENTS_AND_GOVERNANCE.md` — change and claim governance
6. `ALVENQIS_05_DECIZII_RECOMANDATE.md` — accepted decisions versus recommendations

The filenames remain stable to avoid breaking existing links. Their content is
English, and the legacy Romanian words in three filenames do not imply a second
or alternate source set.
