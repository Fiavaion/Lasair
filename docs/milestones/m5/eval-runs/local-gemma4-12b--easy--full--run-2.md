# Network-generation eval

**Model:** `ollama:gemma4:12b` · **Assist:** full · **Retrieval:** semantic · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 19/20 (**95%**) · **Score:** 76/78 points (**97%** mean per prompt) · **Time:** 431 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 3 | img.blur, img.noise, sig.lfo | 0/4 | 19 s |  | 4/4 (100%) |
| gradient-colour | valid | 3 | img.hsv_adjust, img.ramp, sig.lfo | 1/5 | 23 s |  | 5/5 (100%) |
| feedback-trails | valid | 5 | img.composite, img.feedback, img.level, img.noise, img.transform | 3/10 | 37 s |  | 5/5 (100%) |
| kaleido | valid | 3 | img.kaleidoscope, img.noise, sig.timer | 0/4 | 13 s |  | 3/3 (100%) |
| lfo-brightness | valid | 5 | img.constant, img.level, img.render_points, msh.colour, sig.lfo | 5/8 | 34 s |  | 5/5 (100%) |
| edges | valid | 4 | cmp.window, img.edge, img.level, img.noise | 0/4 | 19 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.ramp, img.tile | 0/4 | 16 s |  | 4/4 (100%) |
| two-sources-blend | valid | 5 | img.blend, img.noise, img.ramp, sig.lfo, sig.math | 2/6 | 33 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/2 | 11 s |  | 4/4 (100%) |
| shape-spin | valid | 3 | img.shape, img.transform, sig.timer | 1/5 | 18 s |  | 4/4 (100%) |
| bloom-glow | valid | 6 | img.bloom, img.level, img.shape, img.transform, sig.lfo, sig.timer | 1/5 | 26 s |  | 3/3 (100%) |
| displace | valid | 3 | img.displace, img.noise, img.ramp | 0/4 | 17 s |  | 4/4 (100%) |
| particles | valid | 3 | cmp.window, img.render_points, pnt.particles | 0/4 | 21 s |  | 4/4 (100%) |
| mesh-sphere | valid | 3 | img.render, mat.splat, msh.sphere | 0/4 | 15 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 1/5 | 25 s |  | 6/6 (100%) |
| colour-grade | valid | 5 | img.hsv_adjust, img.level, img.noise, sig.lfo | 0/4 | 20 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 0/4 | 16 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 4 | img.constant, img.mixer, img.noise, img.ramp | 3/5 | 26 s | 4 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | valid | 5 | img.blend, img.noise, img.ramp, img.reaction_diffusion, sig.speed | 1/5 | 26 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 0/4 | 16 s |  | 4/4 (100%) |
