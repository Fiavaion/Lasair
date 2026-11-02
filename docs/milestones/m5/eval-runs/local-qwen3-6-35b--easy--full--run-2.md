# Network-generation eval

**Model:** `ollama:qwen3.6:35b` · **Assist:** full · **Retrieval:** semantic · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 19/20 (**95%**) · **Score:** 76/78 points (**97%** mean per prompt) · **Time:** 212 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 6/9 | 20 s |  | 4/4 (100%) |
| gradient-colour | valid | 2 | img.hsv_adjust, img.ramp | 0/4 | 8 s |  | 5/5 (100%) |
| feedback-trails | valid | 4 | img.composite, img.feedback, img.noise, img.transform | 1/5 | 11 s |  | 5/5 (100%) |
| kaleido | valid | 3 | img.kaleidoscope, img.noise, sig.speed | 0/4 | 6 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 1/5 | 9 s |  | 5/5 (100%) |
| edges | valid | 5 | cmp.window, img.composite, img.constant, img.edge, img.noise | 1/5 | 11 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.ramp, img.tile | 0/4 | 8 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, img.ramp, sig.lfo | 0/4 | 7 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/4 | 9 s |  | 4/4 (100%) |
| shape-spin | valid | 2 | img.shape, img.transform | 0/4 | 8 s |  | 4/4 (100%) |
| bloom-glow | valid | 4 | img.blend, img.bloom, img.shape | 11/13 | 22 s |  | 3/3 (100%) |
| displace | valid | 4 | cmp.window, img.displace, img.noise, img.ramp | 0/4 | 8 s |  | 4/4 (100%) |
| particles | valid | 3 | cmp.window, img.render_points, pnt.particles | 0/4 | 11 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/4 | 6 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 1/5 | 16 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/4 | 11 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 0/4 | 9 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 4 | img.constant, img.mixer, img.noise, img.ramp | 3/5 | 14 s | 4 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | valid | 4 | img.blend, img.ramp, img.reaction_diffusion, img.shape | 0/4 | 8 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 0/4 | 10 s |  | 4/4 (100%) |
