# Network-generation eval

**Model:** `ollama:qwen3.6:35b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 16/20 (**80%**) · **Score:** 71/78 points (**92%** mean per prompt) · **Time:** 547 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/4 | 12 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 2 | img.compute, img.hsv_adjust | 2/24 | 32 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 4 | img.blur, img.composite, img.feedback, img.noise | 0/9 | 12 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/5 | 6 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/7 | 12 s |  | 5/5 (100%) |
| edges | valid | 3 | img.constant, img.edge, img.noise | 0/7 | 8 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 3 | img.compute, img.mirror | 1/40 | 100 s | missing img.tile; 2 node error(s): gradient: line 7, col 39: invalid type for constructor component at index [1] | 2/4 (50%) |
| two-sources-blend | valid | 4 | img.blend, img.constant, img.noise, sig.lfo | 2/13 | 26 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 1/7 | 6 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 0 |  | 0/40 | 185 s | 0 nodes < 1; missing img.shape; nothing is animated | 1/4 (25%) |
| bloom-glow | valid | 3 | img.bloom, img.level, img.noise | 0/6 | 8 s |  | 3/3 (100%) |
| displace | valid | 5 | img.blend, img.constant, img.displace, img.noise | 1/8 | 12 s |  | 4/4 (100%) |
| particles | valid | 8 | img.render_points, pnt.age, pnt.attract, pnt.drag, pnt.emitter, pnt.integrate, pnt.points, pnt.solver | 1/19 | 39 s |  | 4/4 (100%) |
| mesh-sphere | valid | 3 | cmp.window, img.render, msh.iso_surface | 0/12 | 21 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/5 | 7 s |  | 6/6 (100%) |
| colour-grade | valid | 4 | img.hsv_adjust, img.level, img.noise | 0/6 | 8 s |  | 3/3 (100%) |
| lookup-recolour | valid | 5 | img.blend, img.constant, img.lookup, img.noise | 0/15 | 19 s |  | 3/3 (100%) |
| composite-layers | valid | 5 | img.composite, img.fluid, img.nca, img.noise | 1/9 | 15 s |  | 3/3 (100%) |
| reaction-diffusion | valid | 2 | img.fluid, img.hsv_adjust | 0/5 | 7 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 0/7 | 11 s |  | 4/4 (100%) |
