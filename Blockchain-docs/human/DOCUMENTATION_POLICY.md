# Alvenqis Documentation Policy

Status: Accepted workspace policy

This policy prevents historical notes, recommendations, and incomplete product
ideas from being mistaken for current Alvenqis behavior.

## Source-of-truth order

When documents disagree, use this order:

1. validated implementation and consensus tests for current behavior;
2. `Blockchain-docs/internal/memory/DECISIONS.md` for explicitly accepted decisions;
3. `Blockchain-docs/internal/memory/OPEN_QUESTIONS.md` for unresolved decisions;
4. the canonical `source-info/ALVENQIS_00` through `ALVENQIS_05` set;
5. current specifications under `Blockchain-docs/human/` and active component READMEs;
6. roadmaps and task plans for future work;
7. historical reports only as evidence of what was known at their stated date.

No recommendation becomes an accepted protocol decision merely because it is
written in a source-info, roadmap, audit, or website document.

## Document classes

| Class | Meaning | May guide new implementation? | Public web reader |
|---|---|---:|---:|
| Implemented | Matches code and current validation evidence | Yes | Yes |
| Accepted policy | Approved non-code rule or decision | Yes | Yes |
| Candidate | Implemented for controlled Mainnet Candidate use, not public Mainnet | With stated limits | Yes |
| Draft / Planned / Research | Intent or design work that is not complete | No, without review | Yes, with status |
| Historical | Time-bound evidence that may describe removed code | No | No |
| Internal | Agent notes, PR drafts, private planning, or operational scratch data | No | No |

The machine-readable publication rules live in
`documentation-manifest.json`. The generated
`DOCUMENTATION_INVENTORY.md` covers the publishable documentation surface and
tracked component references. Private agent instructions, internal trackers,
local tooling, build outputs, release staging, and generated artifacts are
deliberately excluded.

## Required writing rules

- Write repository documents in English.
- Put one level-one title at the top.
- State maturity near the top when a document describes a product or service.
- Use `Mainnet Candidate`, never `Mainnet Live`, until G4 is approved.
- Use present tense only for behavior verified in current code.
- Use past tense and an explicit historical banner for removed behavior.
- Link to the canonical current document instead of copying mutable facts into
  multiple reports.
- Keep secrets, keys, credentials, tokens, private host inventories, and user
  data out of documentation.
- Do not describe CPU, OpenCL, hybrid, or host-emulated mining as supported.
  Product mining is NVIDIA CUDA-only; host code performs consensus validation,
  not mining search.
- Do not describe Electron as a product path. Tauri is the sole Control Center
  desktop path.

## Historical-document rule

A retained historical document must say, before its first substantive section,
that it is historical, not current guidance, and where the current source lives.
Historical findings must never be silently edited into a fictional record of
what the old audit originally found.

## Web rendering and publication

The website imports only the allowlisted Markdown sources and applies the
internal/historical exclusions from `documentation-manifest.json`. Markdown is
rendered through `react-markdown` with GFM support. Raw HTML in Markdown is not
executed, external links receive safe browser attributes, and local Markdown
links are resolved through the documentation route.

Run:

```bash
node Blockchain-scripts/docs/audit-docs.mjs --write
npm --prefix Blockchain-prototype/alvenqis-website run build
```

The first command checks all repository Markdown files, validates local links
and known stale claims, and refreshes the inventory. The second command proves
that the public reader can bundle and render the allowed documents.
