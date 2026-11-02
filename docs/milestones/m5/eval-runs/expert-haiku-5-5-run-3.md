# Network-generation eval (re-scored)

**Kept projects:** `/private/tmp/claude-501/-Users-mark-Desktop-AIProjects-Atidarla/e979e503-c8fb-4c86-a50b-167f0ded8ddb/scratchpad/keep/expert-haiku-3` · **Prompts:** `docs/milestones/m5/eval-prompts-expert.toml` · **Valid:** 5/12 (**42%**) · **Score:** 223/246 points (**90%** mean per prompt)

Re-scored from the kept networks with the current checker; no model ran.

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| x-cue-steps | **invalid** | 4 | img.level, img.noise, img.transform, sig.timer | 0/25 | 118 s | missing dat.table; no dat.table.rows contains "intro"; no dat.table.rows contains "verse"; no dat.table.rows contains "chorus"; no dat.table.rows contains "outro"; no dat.table.rows contains "0.3"; no dat.table.rows contains "0.6"; no dat.table.rows contains "20"; no dat.table.rows contains "45"; no dat.table.rows contains "90"; img.transform rotate not driven as asked; not met in any accepted way: the brightness follows the current cue, live (a Level brightness or an HSV Adjust value) | 7/19 (37%) |
| x-look-sheet | **invalid** | 6 | dat.select, dat.table, img.blur, img.hsv_adjust, img.noise, img.transform | 0/30 | 74 s | img.hsv_adjust hue not driven as asked; img.transform scale not driven as asked; img.blur size not driven as asked | 17/20 (85%) |
| x-tempo-from-table | **invalid** | 5 | dat.select, dat.table, img.level, img.shape, sig.beat | 0/17 | 37 s | sig.beat bpm not driven as asked | 16/17 (94%) |
| x-debug-tunnel | valid | 14 | cmp.window, img.bloom, img.blur, img.composite, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.transform, sig.lfo | 1/11 | 24 s |  | 23/23 (100%) |
| x-debug-logo-wobble | **invalid** | 12 | cmp.window, img.bloom, img.composite, img.feedback, img.level, img.ramp, img.shape, img.text, img.transform, sig.lfo | 0/18 | 39 s | not met in any accepted way: the badge is on top of the background (the stack's A input comes from the badge) | 23/24 (96%) |
| x-debug-colour-show | valid | 12 | cmp.window, img.bloom, img.blur, img.composite, img.hsv_adjust, img.level, img.noise, img.resolution, img.shape, img.tile, sig.lfo | 0/21 | 47 s |  | 21/21 (100%) |
| x-show-six-changes | valid | 36 | cmp.window, img.bloom, img.blur, img.composite, img.edge, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.ramp, img.shape, img.switch, img.text, img.tile, img.transform, sig.lfo, sig.noise, sig.timer | 0/11 | 16 s |  | 28/28 (100%) |
| x-show-sections-edit | **invalid** | 36 | cmp.window, img.blur, img.composite, img.edge, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.mirror, img.noise, img.ramp, img.shape, img.switch, img.text, img.tile, img.transform, sig.lfo, sig.noise, sig.timer | 0/6 | 20 s | img.switch index every 5 s not driven as asked | 25/26 (96%) |
| x-orbit-dot | valid | 8 | cmp.window, img.composite, img.feedback, img.level, img.shape, sig.lfo, sig.merge | 0/13 | 55 s |  | 16/16 (100%) |
| x-beat-flash | **invalid** | 8 | cmp.window, img.hsv_adjust, img.level, img.noise, img.transform, sig.beat, sig.lfo | 0/8 | 37 s | img.hsv_adjust hue not driven as asked; img.hsv_adjust hue not driven as asked; img.transform scale not driven as asked; img.transform scale not driven as asked | 11/15 (73%) |
| x-economy-mandala | valid | 5 | cmp.window, img.composite, img.kaleidoscope, img.ramp, img.shape | 0/5 | 17 s |  | 19/19 (100%) |
| x-economy-comet | **invalid** | 5 | cmp.window, img.composite, img.feedback, img.shape, img.transform | 0/5 | 20 s | no img.transform.scale >= 1.001 (component 0) | 17/18 (94%) |

