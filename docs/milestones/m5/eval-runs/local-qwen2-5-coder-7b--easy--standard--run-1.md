# Network-generation eval

**Model:** `ollama:qwen2.5-coder:7b` · **Assist:** standard · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 17/20 (**85%**) · **Score:** 75/78 points (**96%** mean per prompt) · **Time:** 510 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/1 | 15 s |  | 4/4 (100%) |
| gradient-colour | valid | 3 | img.hsv_adjust, img.ramp, sig.lfo | 0/1 | 15 s |  | 5/5 (100%) |
| feedback-trails | valid | 4 | img.composite, img.feedback, img.noise, img.transform | 2/2 | 15 s |  | 5/5 (100%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/1 | 10 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 2/3 | 19 s |  | 5/5 (100%) |
| edges | valid | 4 | img.edge, img.level, img.ramp, pnt.noise | 6/4 | 33 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.noise, img.tile | 0/1 | 9 s |  | 4/4 (100%) |
| two-sources-blend | **invalid** | 6 | img.blend, img.level, img.noise, sig.lfo, sig.math, sig.pattern | 5/4 | 58 s | nothing is animated | 3/4 (75%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/1 | 9 s |  | 4/4 (100%) |
| shape-spin | valid | 3 | img.shape, pnt.scatter, pnt.vortex | 4/4 | 34 s |  | 4/4 (100%) |
| bloom-glow | valid | 5 | img.blend, img.bloom, img.noise, img.ramp, img.shape | 9/4 | 39 s |  | 3/3 (100%) |
| displace | **invalid** | 2 | img.displace, img.noise | 3/2 | 12 s | 2 nodes < 3 | 3/4 (75%) |
| particles | valid | 3 | img.noise, img.render_points, pnt.particles | 7/4 | 35 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/1 | 12 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 4/4 | 30 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/1 | 12 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 3/4 | 29 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 6 | img.blend, img.mixer, img.noise | 10/4 | 56 s | missing img.composite | 2/3 (67%) |
| reaction-diffusion | valid | 4 | img.level, img.noise, img.reaction_diffusion | 28/4 | 49 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.blend, img.noise, img.text | 0/1 | 17 s |  | 4/4 (100%) |
