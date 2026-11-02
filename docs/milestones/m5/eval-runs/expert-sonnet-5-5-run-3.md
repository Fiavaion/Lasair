# Network-generation eval (re-scored)

**Kept projects:** `/private/tmp/claude-501/-Users-mark-Desktop-AIProjects-Atidarla/e979e503-c8fb-4c86-a50b-167f0ded8ddb/scratchpad/keep/expert-sonnet-3` · **Prompts:** `docs/milestones/m5/eval-prompts-expert.toml` · **Valid:** 11/12 (**92%**) · **Score:** 245/246 points (**100%** mean per prompt)

Re-scored from the kept networks with the current checker; no model ran.

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| x-cue-steps | valid | 5 | dat.table, img.level, img.noise, img.transform, sig.timer | 0/18 | 40 s |  | 19/19 (100%) |
| x-look-sheet | valid | 5 | dat.table, img.blur, img.hsv_adjust, img.noise, img.transform | 0/6 | 17 s |  | 20/20 (100%) |
| x-tempo-from-table | **invalid** | 5 | dat.table, img.level, img.shape, sig.beat, sig.constant | 1/13 | 28 s | sig.beat bpm not driven as asked | 16/17 (94%) |
| x-debug-tunnel | valid | 14 | cmp.window, img.bloom, img.blur, img.composite, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.transform, sig.lfo | 0/5 | 11 s |  | 23/23 (100%) |
| x-debug-logo-wobble | valid | 12 | cmp.window, img.bloom, img.composite, img.feedback, img.level, img.ramp, img.shape, img.text, img.transform, sig.lfo | 0/13 | 24 s |  | 24/24 (100%) |
| x-debug-colour-show | valid | 12 | cmp.window, img.bloom, img.blur, img.composite, img.hsv_adjust, img.level, img.noise, img.resolution, img.shape, img.tile, sig.lfo | 0/9 | 19 s |  | 21/21 (100%) |
| x-show-six-changes | valid | 36 | cmp.window, img.bloom, img.blur, img.composite, img.edge, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.ramp, img.shape, img.switch, img.text, img.tile, img.transform, sig.lfo, sig.noise, sig.timer | 0/5 | 13 s |  | 28/28 (100%) |
| x-show-sections-edit | valid | 36 | cmp.window, img.blur, img.composite, img.edge, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.mirror, img.noise, img.ramp, img.shape, img.switch, img.text, img.tile, img.transform, sig.lfo, sig.noise, sig.timer | 0/3 | 14 s |  | 26/26 (100%) |
| x-orbit-dot | valid | 8 | cmp.window, img.composite, img.feedback, img.level, img.shape, sig.lfo, sig.merge | 0/7 | 31 s |  | 16/16 (100%) |
| x-beat-flash | valid | 5 | cmp.window, img.hsv_adjust, img.noise, img.transform, sig.beat | 0/5 | 25 s |  | 15/15 (100%) |
| x-economy-mandala | valid | 5 | cmp.window, img.kaleidoscope, img.lookup, img.ramp, img.shape | 0/5 | 15 s |  | 19/19 (100%) |
| x-economy-comet | valid | 5 | cmp.window, img.composite, img.feedback, img.shape, img.transform | 0/5 | 16 s |  | 18/18 (100%) |

