# Network-generation eval

**Model:** `ollama:qwen2.5-coder:7b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 9/20 (**45%**) · **Score:** 51/78 points (**68%** mean per prompt) · **Time:** 3054 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 3 | img.blur, img.noise, sig.lfo | 0/40 | 182 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 3 | sig.lfo, sig.limit, sig.noise | 5/40 | 24 s | missing img.ramp; missing img.hsv_adjust; nothing is animated | 2/5 (40%) |
| feedback-trails | **invalid** | 3 | img.blend, img.feedback, img.noise | 40/45 | 45 s | 3 nodes < 4; missing img.transform; missing img.composite | 2/5 (40%) |
| kaleido | valid | 4 | img.kaleidoscope, img.level, img.noise | 1/40 | 27 s |  | 3/3 (100%) |
| lfo-brightness | **invalid** | 0 |  | 160/243 | 205 s | 0 nodes < 3; missing sig.lfo; missing img.level; nothing is animated | 1/5 (20%) |
| edges | valid | 3 | img.constant, img.edge, img.level | 1/40 | 25 s |  | 3/3 (100%) |
| mirror-tile | valid | 7 | img.mirror, img.tile | 0/40 | 33 s |  | 4/4 (100%) |
| two-sources-blend | **invalid** | 1 | img.noise | 121/362 | 1806 s | 1 nodes < 3; missing img.blend; nothing is animated | 1/4 (25%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/47 | 29 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 0 |  | 38/40 | 47 s | 0 nodes < 1; missing img.shape; nothing is animated | 1/4 (25%) |
| bloom-glow | valid | 4 | img.bloom, img.blur, img.level, img.noise | 0/40 | 21 s |  | 3/3 (100%) |
| displace | **invalid** | 2 | img.displace, img.noise | 2/40 | 25 s | 2 nodes < 3 | 3/4 (75%) |
| particles | **invalid** | 0 |  | 2/40 | 59 s | 0 nodes < 2; missing pnt.; missing img.render/img.render_points | 1/4 (25%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 35/40 | 34 s |  | 4/4 (100%) |
| math-channels | **invalid** | 3 | img.noise, img.transform, sig.lfo | 1/49 | 30 s | 3 nodes < 4; missing sig.math; nothing is animated | 3/6 (50%) |
| colour-grade | valid | 4 | img.bloom, img.blur, img.hsv_adjust, img.level | 0/40 | 36 s |  | 3/3 (100%) |
| lookup-recolour | **invalid** | 2 | img.lookup, img.noise | 0/40 | 20 s | 2 nodes < 3 | 2/3 (67%) |
| composite-layers | **invalid** | 0 |  | 0/40 | 111 s | 0 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | **invalid** | 1 | img.reaction_diffusion | 80/202 | 263 s | 1 nodes < 2 | 2/3 (67%) |
| text-overlay | valid | 2 | img.noise, img.text | 18/40 | 32 s |  | 4/4 (100%) |
