# Network-generation eval (re-scored)

**Kept projects:** `/private/tmp/claude-501/-Users-mark-Desktop-AIProjects-Atidarla/e979e503-c8fb-4c86-a50b-167f0ded8ddb/scratchpad/keep/hard-haiku-1` · **Prompts:** `docs/milestones/m5/eval-prompts-hard.toml` · **Valid:** 14/15 (**93%**) · **Score:** 231/233 points (**99%** mean per prompt)

Re-scored from the kept networks with the current checker; no model ran.

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| h-tunnel-two-screens | valid | 10 | cmp.window, img.bloom, img.composite, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.transform | 0/13 | 23 s |  | 23/23 (100%) |
| h-layered-stage | valid | 7 | cmp.window, img.composite, img.noise, img.ramp, img.shape, img.tile | 0/7 | 18 s |  | 17/17 (100%) |
| h-three-looks | valid | 6 | cmp.window, img.noise, img.ramp, img.shape, img.switch, sig.timer | 0/8 | 21 s |  | 17/17 (100%) |
| h-torus-dust | valid | 9 | img.render, mat.point_sprite, msh.scatter, msh.torus, pnt.colour, pnt.drag, pnt.integrate, pnt.solver, pnt.turbulence | 0/14 | 36 s |  | 17/17 (100%) |
| h-spark-fountain | valid | 3 | img.bloom, img.render_points, pnt.particles | 0/9 | 17 s |  | 13/13 (100%) |
| h-orbit-cam | valid | 5 | img.render, mat.toon, mat.unlit, msh.sphere, msh.torus | 0/12 | 14 s |  | 19/19 (100%) |
| h-breathing-clouds | valid | 7 | img.hsv_adjust, img.noise, img.transform, sig.constant, sig.lfo, sig.noise, sig.speed | 0/11 | 30 s |  | 10/10 (100%) |
| h-cue-sheet | **invalid** | 5 | dat.select, dat.table, img.level, img.noise, img.transform | 2/32 | 58 s | img.level brightness from dat.table not driven as asked; img.transform rotate from dat.table not driven as asked | 15/17 (88%) |
| h-worn-tape | valid | 6 | img.blend, img.blur, img.feedback, img.level, img.noise, img.transform | 0/4 | 19 s |  | 9/9 (100%) |
| h-stark-poster | valid | 5 | img.lookup, img.mirror, img.noise, img.ramp, img.threshold | 0/6 | 17 s |  | 13/13 (100%) |
| h-fix-broken-chain | valid | 7 | cmp.window, img.bloom, img.blur, img.noise, img.resolution, img.transform, sig.lfo | 0/3 | 9 s |  | 14/14 (100%) |
| h-share-one-source | valid | 8 | cmp.window, img.blur, img.composite, img.hsv_adjust, img.level, img.noise | 0/3 | 5 s |  | 16/16 (100%) |
| h-fix-trails | valid | 6 | cmp.window, img.composite, img.feedback, img.level, img.noise, img.transform | 0/5 | 15 s |  | 16/16 (100%) |
| h-show-change-request | valid | 17 | cmp.window, img.blend, img.bloom, img.blur, img.composite, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.shape, img.text, img.transform, sig.lfo, sig.noise | 0/5 | 8 s |  | 20/20 (100%) |
| h-wire-the-signals | valid | 6 | cmp.window, img.hsv_adjust, img.level, img.noise, sig.lfo, sig.noise | 0/4 | 8 s |  | 12/12 (100%) |

