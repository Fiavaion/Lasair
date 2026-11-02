# Network-generation eval

**Model:** `ollama:qwen3.6:35b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 16/20 (**80%**) · **Score:** 72/78 points (**93%** mean per prompt) · **Time:** 547 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/4 | 14 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 3 | img.constant, img.hsv_adjust, sig.lfo | 0/6 | 9 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 4 | img.blur, img.composite, img.feedback, img.noise | 0/8 | 9 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/5 | 6 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/7 | 10 s |  | 5/5 (100%) |
| edges | valid | 3 | img.constant, img.edge, img.noise | 0/8 | 10 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 4 | img.kaleidoscope, img.mirror, img.noise | 1/26 | 36 s | missing img.tile | 3/4 (75%) |
| two-sources-blend | valid | 4 | img.blend, img.constant, img.noise, sig.lfo | 4/12 | 17 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/5 | 5 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 0 |  | 0/40 | 135 s | 0 nodes < 1; missing img.shape; nothing is animated | 1/4 (25%) |
| bloom-glow | valid | 6 | cmp.window, img.blend, img.bloom, img.noise, img.shape | 5/32 | 66 s |  | 3/3 (100%) |
| displace | valid | 6 | img.blend, img.compute, img.constant, img.displace, img.noise | 2/38 | 59 s |  | 4/4 (100%) |
| particles | valid | 2 | img.render_points, pnt.particles | 1/15 | 28 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.iso_surface | 2/19 | 33 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/19 | 23 s |  | 6/6 (100%) |
| colour-grade | valid | 4 | img.hsv_adjust, img.level, img.noise | 0/6 | 8 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.compute, img.lookup, img.noise | 1/22 | 39 s |  | 3/3 (100%) |
| composite-layers | valid | 5 | img.composite, img.fluid, img.nca, img.noise | 1/11 | 17 s |  | 3/3 (100%) |
| reaction-diffusion | valid | 2 | img.fluid, img.hsv_adjust | 0/5 | 7 s |  | 3/3 (100%) |
| text-overlay | valid | 2 | img.noise, img.text | 0/8 | 14 s |  | 4/4 (100%) |
