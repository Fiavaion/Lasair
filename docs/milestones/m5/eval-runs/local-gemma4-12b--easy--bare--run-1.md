# Network-generation eval

**Model:** `ollama:gemma4:12b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 7/20 (**35%**) · **Score:** 60/78 points (**79%** mean per prompt) · **Time:** 2286 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 4 | img.blur, img.composite, img.constant, img.noise | 0/23 | 148 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 4 | img.constant, img.hsv_adjust, sig.lfo | 0/40 | 143 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 4 | img.composite, img.feedback, img.hsv_adjust, img.noise | 0/40 | 203 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.nca | 0/6 | 17 s |  | 3/3 (100%) |
| lfo-brightness | **invalid** | 5 | img.constant, sig.constant, sig.lfo, sig.math | 1/40 | 190 s | missing img.level; 1 node error(s): final_color: expression for `colour`: op('mix_brightness'): no such channel; it has: lfo | 3/5 (60%) |
| edges | valid | 2 | img.edge, img.noise | 0/8 | 21 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 5 | img.constant, img.kernel, img.mirror | 0/11 | 54 s | missing img.tile | 3/4 (75%) |
| two-sources-blend | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/23 | 206 s | missing img.blend; nothing is animated | 2/4 (50%) |
| threshold | **invalid** | 4 | img.composite, img.constant, img.displace, img.noise | 0/26 | 107 s | missing img.threshold; missing img.blur | 2/4 (50%) |
| shape-spin | **invalid** | 2 | img.constant, img.kernel | 0/10 | 33 s | missing img.shape; nothing is animated | 2/4 (50%) |
| bloom-glow | valid | 2 | img.bloom, img.kernel | 0/6 | 17 s |  | 3/3 (100%) |
| displace | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/19 | 138 s | missing img.displace | 3/4 (75%) |
| particles | valid | 2 | img.render_points, pnt.particles | 0/10 | 26 s |  | 4/4 (100%) |
| mesh-sphere | **invalid** | 3 | cmp.window, img.render_points, msh.iso_surface | 2/12 | 28 s | missing img.render | 3/4 (75%) |
| math-channels | **invalid** | 3 | img.noise, sig.lfo, sig.math | 0/8 | 21 s | 3 nodes < 4; missing img.transform | 4/6 (67%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/12 | 38 s |  | 3/3 (100%) |
| lookup-recolour | **invalid** | 15 | img.compute, img.constant, img.lookup, img.noise | 0/40 | 367 s | 1 node error(s): lut_stack: line 1, col 13: `in0\n#bind` is not an input socket (in0 to in3) | 2/3 (67%) |
| composite-layers | valid | 5 | img.composite, img.constant | 0/40 | 280 s |  | 3/3 (100%) |
| reaction-diffusion | **invalid** | 4 | img.compute, img.constant, img.feedback, img.lookup | 0/4 | 38 s | 1 node error(s): rd_sim: line 8, col 18: expected `;`, found "as" | 2/3 (67%) |
| text-overlay | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/26 | 210 s | missing img.text | 3/4 (75%) |
