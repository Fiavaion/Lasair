# Network-generation eval

**Model:** `openai:Qwen3.6-35B-A3B-NVFP4` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 17/20 (**85%**) · **Score:** 75/78 points (**96%** mean per prompt) · **Time:** 647 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/6 | 8 s |  | 4/4 (100%) |
| gradient-colour | valid | 2 | img.hsv_adjust, img.ramp | 1/9 | 10 s |  | 5/5 (100%) |
| feedback-trails | **invalid** | 5 | img.blur, img.composite, img.feedback, img.level, img.noise | 0/9 | 11 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/6 | 7 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/8 | 10 s |  | 5/5 (100%) |
| edges | valid | 3 | img.edge, img.level, img.noise | 0/8 | 11 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.ramp, img.tile | 2/10 | 11 s |  | 4/4 (100%) |
| two-sources-blend | valid | 6 | img.blend, img.channel_image, img.noise, sig.constant, sig.lfo, sig.speed | 5/25 | 44 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/7 | 7 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 3 | img.render, msh.circle, msh.transform | 4/21 | 31 s | missing img.shape | 3/4 (75%) |
| bloom-glow | valid | 28 | img.bloom, img.composite, img.constant, img.level, img.shape, sig.lfo, sig.merge | 6/39 | 94 s |  | 3/3 (100%) |
| displace | valid | 3 | img.compute, img.displace, img.noise | 1/13 | 20 s |  | 4/4 (100%) |
| particles | valid | 2 | img.render_points, pnt.particles | 1/13 | 18 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.iso_surface | 2/23 | 32 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/8 | 11 s |  | 6/6 (100%) |
| colour-grade | valid | 4 | img.hsv_adjust, img.level, img.noise | 0/7 | 10 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.compute, img.lookup, img.noise | 0/29 | 42 s |  | 3/3 (100%) |
| composite-layers | valid | 6 | img.composite, img.fluid, img.level, img.nca, img.noise | 1/13 | 22 s |  | 3/3 (100%) |
| reaction-diffusion | **invalid** | 7 | img.channel_mix, img.compute, img.constant, img.feedback, img.hsv_adjust, img.level | 0/40 | 235 s | 1 node error(s): reactor: line 42, col 28: wrong type passed as argument #1 to `length` | 2/3 (67%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 0/9 | 12 s |  | 4/4 (100%) |
