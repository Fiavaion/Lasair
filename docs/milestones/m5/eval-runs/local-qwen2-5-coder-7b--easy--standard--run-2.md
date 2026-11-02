# Network-generation eval

**Model:** `ollama:qwen2.5-coder:7b` · **Assist:** standard · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 18/20 (**90%**) · **Score:** 76/78 points (**97%** mean per prompt) · **Time:** 453 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/1 | 8 s |  | 4/4 (100%) |
| gradient-colour | valid | 3 | img.hsv_adjust, img.ramp, sig.lfo | 0/1 | 12 s |  | 5/5 (100%) |
| feedback-trails | valid | 4 | img.composite, img.feedback, img.noise, img.transform | 4/4 | 30 s |  | 5/5 (100%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/1 | 7 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 2/3 | 15 s |  | 5/5 (100%) |
| edges | valid | 4 | cmp.window, img.edge, img.level, img.noise | 0/1 | 11 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.noise, img.tile | 0/1 | 10 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, img.ramp, sig.lfo | 4/4 | 33 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/1 | 6 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 5 | img.shape, pnt.attribute_noise, pnt.scatter, pnt.vortex, sig.lfo | 6/4 | 39 s | 2 node error(s): vortex1: expression for `centre`: op('shape1'): no such channel | 3/4 (75%) |
| bloom-glow | valid | 5 | img.blend, img.bloom, img.noise, img.shape, img.transform | 5/4 | 26 s |  | 3/3 (100%) |
| displace | valid | 3 | img.displace, img.noise, img.ramp | 8/4 | 24 s |  | 4/4 (100%) |
| particles | valid | 5 | cmp.window, img.constant, img.noise, img.render_points, pnt.particles | 10/4 | 43 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/1 | 8 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 4/4 | 29 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/1 | 11 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 1/2 | 16 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 8 | img.blend, img.layer, img.noise | 4/4 | 79 s | missing img.composite | 2/3 (67%) |
| reaction-diffusion | valid | 3 | img.noise, img.reaction_diffusion, pnt.colour | 16/4 | 37 s |  | 3/3 (100%) |
| text-overlay | valid | 2 | img.noise, img.text | 0/1 | 10 s |  | 4/4 (100%) |
