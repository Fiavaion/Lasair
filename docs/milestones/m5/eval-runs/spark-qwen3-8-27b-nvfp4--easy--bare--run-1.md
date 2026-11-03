# Network-generation eval

**Model:** `openai:Qwen3.8-27B-NVFP4` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 17/20 (**85%**) · **Score:** 74/78 points (**94%** mean per prompt) · **Time:** 1338 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/5 | 26 s |  | 4/4 (100%) |
| gradient-colour | valid | 2 | img.hsv_adjust, img.ramp | 2/10 | 52 s |  | 5/5 (100%) |
| feedback-trails | **invalid** | 5 | img.composite, img.displace, img.feedback, img.level, img.noise | 0/9 | 62 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/6 | 36 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 1/8 | 44 s |  | 5/5 (100%) |
| edges | valid | 2 | img.edge, img.noise | 0/6 | 34 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.kernel, img.mirror, img.tile | 1/13 | 64 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, img.ramp, sig.lfo | 2/12 | 68 s |  | 4/4 (100%) |
| threshold | **invalid** | 3 | img.blur, img.level, img.noise | 0/7 | 44 s | missing img.threshold | 3/4 (75%) |
| shape-spin | valid | 2 | img.shape, img.transform | 0/7 | 35 s |  | 4/4 (100%) |
| bloom-glow | valid | 8 | img.bloom, img.composite, img.constant, img.hsv_adjust, img.level, img.noise | 0/13 | 135 s |  | 3/3 (100%) |
| displace | valid | 3 | img.displace, img.noise, img.ramp | 2/11 | 58 s |  | 4/4 (100%) |
| particles | valid | 2 | img.render_points, pnt.particles | 1/10 | 54 s |  | 4/4 (100%) |
| mesh-sphere | valid | 3 | img.render, mat.pbr, msh.sphere | 0/11 | 58 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/9 | 54 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/7 | 47 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 2/13 | 72 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 0 |  | 10/79 | 233 s | 0 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | valid | 5 | img.bloom, img.kernel, img.level, img.noise, img.reaction_diffusion | 1/15 | 120 s |  | 3/3 (100%) |
| text-overlay | valid | 2 | img.noise, img.text | 0/7 | 42 s |  | 4/4 (100%) |
