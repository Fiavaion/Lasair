# Network-generation eval

**Model:** `openai:Qwen3.6-35B-A3B-NVFP4` · **Assist:** full · **Retrieval:** semantic · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 18/20 (**90%**) · **Score:** 75/78 points (**95%** mean per prompt) · **Time:** 520 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/4 | 7 s |  | 4/4 (100%) |
| gradient-colour | valid | 2 | img.hsv_adjust, img.ramp | 0/4 | 10 s |  | 5/5 (100%) |
| feedback-trails | valid | 4 | img.composite, img.feedback, img.noise, img.transform | 0/4 | 29 s |  | 5/5 (100%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/4 | 6 s |  | 3/3 (100%) |
| lfo-brightness | valid | 4 | img.constant, img.level, sig.lfo, sig.math | 0/4 | 11 s |  | 5/5 (100%) |
| edges | valid | 6 | cmp.window, img.composite, img.constant, img.edge, img.noise | 1/5 | 20 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.ramp, img.tile | 0/2 | 59 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, img.ramp, sig.lfo | 0/2 | 62 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/4 | 8 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 3 | img.render, msh.circle, pnt.twist | 6/9 | 16 s | missing img.shape | 3/4 (75%) |
| bloom-glow | valid | 3 | img.bloom, img.shape, img.transform | 7/13 | 24 s |  | 3/3 (100%) |
| displace | valid | 4 | cmp.window, img.displace, img.noise, img.ramp | 0/2 | 60 s |  | 4/4 (100%) |
| particles | valid | 3 | cmp.window, img.render_points, pnt.particles | 0/4 | 11 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/2 | 58 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 1/3 | 64 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/4 | 17 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 0/4 | 15 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 4 | img.mixer, img.noise, img.ramp | 0/4 | 13 s | 4 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | valid | 3 | img.constant, img.noise, img.reaction_diffusion | 7/8 | 18 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 1/5 | 12 s |  | 4/4 (100%) |
