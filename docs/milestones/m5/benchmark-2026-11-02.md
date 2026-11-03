# Assistant benchmark, 2026-11-01 to 2026-11-02

`lasair agent eval` on three prompt sets, three runs per cell. A prompt is
**valid** when it scores every check; **checks** is the share of points
scored over the whole run (partial credit). Cloud: Anthropic API. Local:
Ollama on an Apple M5 Pro, 48 GB, `think: false`. Hard and expert runs were
re-scored with `lasair agent judge` after the checker fixes, so every run of
one set is graded by the same checker. Reports: [`eval-runs/`](eval-runs/).
Easy cloud runs predate the Score column (valid only).

Assist levels (PRD A72): **bare** is the plain tool loop; **standard** is the
app's behaviour (operator list, planned mode, repair prompts); **full** adds
semantic retrieval of operator docs and examples, plan validation, 7 rounds
and a self-review.

## Easy set (20 prompts)

| Model | Assist | Valid per run | Checks per run | Median s/prompt |
|---|---|---|---|---|
| Haiku 5.5 | standard | 20 / 20 / 20 | - | 5 |
| Sonnet 5.5 | standard | 20 / 20 / 20 | - | 6 |
| Opus 5.5 | standard | 20 / 20 / 20 | - | 6 |
| Qwen3.6 35B | bare | 16 / 16 / 13 | 91 / 92 / 86% | 14 |
| Qwen3.6 35B | standard | 19 / 19 / 18 | 99 / 99 / 97% | 5 |
| Qwen3.6 35B | full | 18 / 19 / 19 | 96 / 97 / 99% | 9 |
| Gemma 4 12B | bare | 7 / 8 / 7 | 77 / 74 / 73% | 51 |
| Gemma 4 12B | standard | 19 / 19 / 19 | 97 / 97 / 97% | 11 |
| Gemma 4 12B | full | 19 / 19 / 19 | 99 / 97 / 97% | 20 |
| Qwen2.5 Coder 7B | bare | 9 / 9 / 7 | 65 / 72 / 63% | 33 |
| Qwen2.5 Coder 7B | standard | 17 / 18 / 19 | 96 / 97 / 99% | 16 |
| Qwen2.5 Coder 7B | full | 17 / 17 / 18 | 96 / 96 / 97% | 15 |

## Hard set (15 prompts, 233 checks)

| Model | Assist | Valid per run | Checks per run | Median s/prompt |
|---|---|---|---|---|
| Sonnet 5.5 | standard | 15 / 15 / 15 | 100 / 100 / 100% | 12 |
| Haiku 5.5 | standard | 14 / 14 / 14 | 99 / 99 / 99% | 17 |
| Qwen3.6 35B | bare | 2 / 4 / 3 | 65 / 69 / 71% | 34 |
| Qwen3.6 35B | standard | 3 / 4 / 4 | 68 / 75 / 76% | 15 |
| Qwen3.6 35B | full | 6 / 7 / 6 | 82 / 88 / 84% | 17 |
| Gemma 4 12B | bare | 2 / 2 / 2 | 54 / 55 / 50% | 47 |
| Gemma 4 12B | standard | 5 / 5 / 5 | 73 / 73 / 72% | 22 |
| Gemma 4 12B | full | 3 / 3 / 3 | 81 / 79 / 78% | 35 |
| Qwen2.5 Coder 7B | bare | 0 / 0 / 0 | 39 / 38 / 38% | 48 |
| Qwen2.5 Coder 7B | standard | 1 / 1 / 2 | 59 / 58 / 59% | 46 |
| Qwen2.5 Coder 7B | full | 2 / 2 / 3 | 76 / 70 / 71% | 52 |

Haiku's one miss every run is `h-cue-sheet` (copies the table's numbers
instead of reading the rows live).

## Expert set (12 prompts, behavioural checks)

| Model | Assist | Valid per run | Checks per run | Median s/prompt |
|---|---|---|---|---|
| Sonnet 5.5 | standard | 12 / 12 / 11 | 100 / 100 / 100% | 18 |
| Haiku 5.5 | standard | 8 / 10 / 5 | 95 / 98 / 91% | 36 |

| Model | Assist | Valid per run | Checks per run | Median s/prompt |
|---|---|---|---|---|
| Qwen3.6 35B | bare | 2 / 1 / 2 | 63 / 69 / 68% | 64 |
| Qwen3.6 35B | standard | 0 / 0 / 0 | 58 / 60 / 65% | 16 |
| Qwen3.6 35B | full | 0 / 0 / 0 | 67 / 72 / 66% | 26 |
| Gemma 4 12B | bare | 0 / 0 / 0 | 54 / 55 / 54% | 68 |
| Gemma 4 12B | standard | 0 / 0 / 0 | 69 / 68 / 68% | 36 |
| Gemma 4 12B | full | 1 / 1 / 1 | 67 / 72 / 71% | 48 |
| Qwen2.5 Coder 7B | bare | 0 / 0 / 0 | 45 / 43 / 43% | 52 |
| Qwen2.5 Coder 7B | standard | 0 / 0 / 0 | 61 / 63 / 61% | 54 |
| Qwen2.5 Coder 7B | full | 0 / 0 / 0 | 67 / 65 / 66% | 70 |

Qwen3.6 35B standard run 2 first stopped with a GPU validation panic (an
MSH readback dropped mid-flight; fixed in fb3ee99); the row above is its
re-run on the fixed build.

Every local valid patch on the expert set is a debugging prompt, which
starts from a seeded patch: `x-debug-tunnel` (Gemma full ×3, Qwen3.6 bare
×3), `x-debug-logo-wobble` and `x-debug-colour-show` (Qwen3.6 bare, once
each). Qwen3.6 fixes the tunnel in every bare run (the tool loop: targeted
edits) and in no standard or full run (planned mode: the whole network
rewritten as one plan). Open question for the app: for an edit to an
existing patch, prefer the tool loop over a planned rebuild, at least on
local models.

Opus 5.5 was not run on the hard or expert sets: Sonnet
already scores every check, so a harder tier is needed before Opus can show
a difference.

## DGX Spark (2026-11-02 to 2026-11-03)

The same three sets on two DGX Sparks over Tailscale (vLLM, NVFP4
weights, `--provider openai`, thinking off), three runs per cell. Qwen3.6
35B (A3B) on one Spark, Qwen3.8 27B (dense) on the other, one eval at a
time per Spark. The Sparks were shared with another machine's jobs, so
times are under shared load and only comparable within this table.
Reports: `eval-runs/spark-*.md`.

| Model | Set | Assist | Valid per run | Checks per run | Median s/prompt |
|---|---|---|---|---|---|
| Qwen3.6 35B | easy | bare | 17 / 14 / 14 | 96 / 89 / 86% | 16 |
| Qwen3.6 35B | easy | standard | 18 / 19 / 18 | 95 / 97 / 96% | 4 |
| Qwen3.6 35B | easy | full | 19 / 19 / 18 | 97 / 97 / 95% | 13 |
| Qwen3.6 35B | hard | bare | 3 / 4 / 4 | 70 / 70 / 69% | 48 |
| Qwen3.6 35B | hard | standard | 4 / 4 / 5 | 73 / 72 / 74% | 12 |
| Qwen3.6 35B | hard | full | 7 / 8 / 7 | 83 / 86 / 81% | 41 |
| Qwen3.6 35B | expert | bare | 1 / 2 / 2 | 62 / 52 / 60% | 65 |
| Qwen3.6 35B | expert | standard | 0 / 0 / 0 | 66 / 70 / 68% | 26 |
| Qwen3.6 35B | expert | full | 1 / 1 / 1 | 84 / 75 / 79% | 56 |
| Qwen3.8 27B | easy | standard | 19 / 20 / 19 | 97 / 100 / 97% | 18 |
| Qwen3.8 27B | easy | full | 19 / 19 / 19 | 97 / 97 / 97% | 40 |
| Qwen3.8 27B | hard | standard | 4 / 4 / 4 | 77 / 76 / 75% | 57 |
| Qwen3.8 27B | hard | full | 6 / 6 / 6 | 83 / 83 / 82% | 99 |
| Qwen3.8 27B | expert | standard | 2 / 2 / 1 | 72 / 68 / 70% | 106 |
| Qwen3.8 27B | expert | full | 2 / 3 / 2 | 83 / 85 / 82% | 221 |

Expert prompts passed: Qwen3.8 full `x-show-six-changes` ×3,
`x-economy-mandala` ×2, `x-debug-tunnel` and `x-show-sections-edit` once;
Qwen3.8 standard `x-show-sections-edit` ×3, `x-debug-tunnel` ×2. Qwen3.6
full `x-show-six-changes` ×2, `x-debug-tunnel` once; Qwen3.6 bare the three
debugging prompts, as on the Mac (`x-debug-tunnel` every run).

- Both Spark models pass expert prompts that are not debugging a seeded
  patch (mostly `x-show-six-changes`), which no model on the Mac did.
  Qwen3.8 27B is the best non-cloud model on the expert set (1–3 of 12
  valid, 82–85% of checks with full help), still far from Haiku (5–10 of
  12), and about 4× slower per prompt than Qwen3.6 on the same hardware (median
  12 vs 44 tokens/s per request).
- Qwen3.6 35B on the Spark matches the Mac on the easy and hard sets, and
  scores higher with full help on the expert set (1/12 valid every run,
  75–84% of checks, against 0/12 and 66–72% on the Mac). The weights differ
  (NVFP4 under vLLM against Ollama's quantisation), so this is not a
  hardware result.
- Ranking holds: full help beats standard on every hard and expert cell
  for both models; bare stays worst on hard and expert.
- No run was scored while a server was down: 21 runs were set aside and
  re-run, all from one bug (fixed in 5bdf493: the OpenAI history sent
  earlier tool-call arguments as objects, which vLLM rejects with 400).
  Every request after the fix succeeded (Qwen3.6 3,994, Qwen3.8 732).

GLM 5.3 Flash and DeepSeek V4 Flash (both Sparks together, 2026-11-03
daytime windows) follow below when run.

## Findings

- The easy set does not separate the cloud models, and the hard set barely
  does. The expert set does: Sonnet 35/36 prompts valid, Haiku 23/36, and
  Haiku varies from run to run. Haiku's misses are near-misses (91–98% of
  checks), mostly copying values where the prompt asks for a live link
  (`x-cue-steps`, `x-look-sheet`, `x-tempo-from-table`) and beat timing.
- Help matters most locally. Bare to standard is the largest step (the
  model learns the operator ids instead of building look-alikes from the
  wrong nodes); full adds 6–13 points of checks on the hard set and nothing
  on the easy set, at up to twice the time per prompt.
- Best local setup: Qwen3.6 35B with full help (82–88% of hard checks,
  6–7/15 valid). Gemma 4 12B is never ahead of it and is slower. No local
  model is reliable on multi-part scenes (`h-layered-stage`,
  `h-torus-dust`, `h-tunnel-two-screens`, `h-stark-poster`); all do well on
  edits and repairs (`h-share-one-source`, `h-wire-the-signals`,
  `h-show-change-request`, `h-fix-broken-chain`).
- The expert set is beyond every local model (0–2 of 12 valid per run, 43–72%
  of checks); their misses are the behaviours the tier tests: stepping cues
  over time, live links to a table, closed feedback loops.
- Bare-mode failures were audited against the kept networks: they are real
  (wrong or invented operators), not checker artifacts.
