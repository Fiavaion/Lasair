# Network-generation eval

**Model:** `openai:Qwen3.8-27B-NVFP4` · **Assist:** standard · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 19/20 (**95%**) · **Score:** 76/78 points (**97%** mean per prompt) · **Time:** 405 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/1 | 8 s |  | 4/4 (100%) |
| gradient-colour | valid | 2 | img.hsv_adjust, img.ramp | 0/1 | 12 s |  | 5/5 (100%) |
| feedback-trails | valid | 4 | img.composite, img.feedback, img.noise, img.transform | 0/1 | 32 s |  | 5/5 (100%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/1 | 8 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/1 | 21 s |  | 5/5 (100%) |
| edges | valid | 4 | cmp.window, img.edge, img.level, img.noise | 0/1 | 15 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.ramp, img.tile | 0/1 | 17 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, img.ramp, sig.lfo | 0/1 | 48 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/1 | 26 s |  | 4/4 (100%) |
| shape-spin | valid | 2 | img.shape, img.transform | 0/1 | 10 s |  | 4/4 (100%) |
| bloom-glow | valid | 6 | img.bloom, img.composite, img.shape | 0/1 | 47 s |  | 3/3 (100%) |
| displace | valid | 4 | cmp.window, img.displace, img.noise, img.ramp | 0/1 | 11 s |  | 4/4 (100%) |
| particles | valid | 2 | img.render_points, pnt.particles | 0/1 | 16 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/1 | 7 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/1 | 18 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/1 | 29 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 0/1 | 24 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 4 | img.constant, img.mixer, img.noise, img.ramp | 0/1 | 18 s | 4 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | valid | 3 | img.level, img.noise, img.reaction_diffusion | 0/1 | 17 s |  | 3/3 (100%) |
| text-overlay | valid | 2 | img.noise, img.text | 0/1 | 20 s |  | 4/4 (100%) |
