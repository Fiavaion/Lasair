# Network-generation eval

**Model:** `openai:Qwen3.6-35B-A3B-NVFP4` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 14/20 (**70%**) · **Score:** 67/78 points (**86%** mean per prompt) · **Time:** 526 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/7 | 9 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 2 | img.compute, img.hsv_adjust | 2/10 | 13 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 6 | img.blur, img.composite, img.feedback, img.hsv_adjust, img.level, img.noise | 0/10 | 18 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/8 | 12 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/7 | 7 s |  | 5/5 (100%) |
| edges | valid | 3 | img.edge, img.level, img.noise | 0/9 | 12 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 1 | img.compute | 1/10 | 16 s | 1 nodes < 3; missing img.mirror; missing img.tile | 1/4 (25%) |
| two-sources-blend | valid | 4 | img.blend, img.constant, img.noise, sig.lfo | 2/12 | 15 s |  | 4/4 (100%) |
| threshold | **invalid** | 3 | img.blur, img.level, img.noise | 0/6 | 8 s | missing img.threshold | 3/4 (75%) |
| shape-spin | **invalid** | 0 |  | 28/40 | 54 s | 0 nodes < 1; missing img.shape; nothing is animated | 1/4 (25%) |
| bloom-glow | valid | 6 | img.blend, img.bloom, img.composite, img.hsv_adjust, img.shape | 2/19 | 29 s |  | 3/3 (100%) |
| displace | valid | 5 | img.blend, img.constant, img.displace, img.noise | 1/12 | 22 s |  | 4/4 (100%) |
| particles | valid | 2 | img.render_points, pnt.particles | 0/17 | 25 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.iso_surface | 0/15 | 21 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/10 | 16 s |  | 6/6 (100%) |
| colour-grade | valid | 4 | img.hsv_adjust, img.level, img.noise | 0/7 | 10 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.compute, img.lookup, img.noise | 0/21 | 44 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 4 | img.mixer, img.noise | 0/11 | 15 s | 4 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | valid | 4 | img.channel_mix, img.compute, img.feedback, img.level | 1/26 | 166 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 0/9 | 14 s |  | 4/4 (100%) |
