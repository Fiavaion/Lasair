# Network-generation eval

**Model:** `ollama:qwen3.6:35b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 13/20 (**65%**) · **Score:** 67/78 points (**86%** mean per prompt) · **Time:** 1521 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/4 | 12 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 4 | img.constant, img.hsv_adjust, sig.lfo | 4/18 | 22 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 4 | img.blur, img.composite, img.feedback, img.noise | 0/8 | 10 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/5 | 6 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/7 | 11 s |  | 5/5 (100%) |
| edges | valid | 3 | img.constant, img.edge, img.noise | 0/7 | 9 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 4 | img.compute, img.mirror, img.transform | 2/31 | 53 s | missing img.tile | 3/4 (75%) |
| two-sources-blend | valid | 4 | img.blend, img.constant, img.noise, sig.lfo | 1/9 | 13 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/2 | 4 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 0 |  | 0/41 | 250 s | 0 nodes < 1; missing img.shape; nothing is animated | 1/4 (25%) |
| bloom-glow | valid | 3 | img.bloom, img.level, img.noise | 0/6 | 8 s |  | 3/3 (100%) |
| displace | **invalid** | 3 | img.compute, img.displace, img.noise | 2/42 | 79 s | 1 node error(s): gradient: line 6, col 47: expected `)`, found "_f32" | 3/4 (75%) |
| particles | **invalid** | 0 |  | 1/43 | 716 s | 0 nodes < 2; missing pnt.; missing img.render/img.render_points | 1/4 (25%) |
| mesh-sphere | valid | 2 | img.render, msh.iso_surface | 0/9 | 17 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/12 | 13 s |  | 6/6 (100%) |
| colour-grade | valid | 4 | img.hsv_adjust, img.level, img.noise | 0/6 | 8 s |  | 3/3 (100%) |
| lookup-recolour | valid | 7 | img.blend, img.channel_image, img.constant, img.lookup, img.noise, sig.pattern | 1/27 | 40 s |  | 3/3 (100%) |
| composite-layers | valid | 5 | img.composite, img.constant, img.fluid, img.noise | 1/10 | 14 s |  | 3/3 (100%) |
| reaction-diffusion | **invalid** | 5 | img.compute, img.feedback, img.layer | 2/40 | 223 s | 3 node error(s): seed: line 8, col 27: inconsistent type passed as argument #2 to `dot` | 2/3 (67%) |
| text-overlay | valid | 2 | img.noise, img.text | 0/8 | 15 s |  | 4/4 (100%) |
