# Network-generation eval

**Model:** `ollama:qwen2.5-coder:7b` · **Assist:** full · **Retrieval:** semantic · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 17/20 (**85%**) · **Score:** 75/78 points (**96%** mean per prompt) · **Time:** 532 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/4 | 13 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 4 | img.blend, img.ramp, sig.lfo, sig.timer | 7/13 | 37 s | missing img.hsv_adjust | 4/5 (80%) |
| feedback-trails | valid | 7 | img.composite, img.feedback, img.noise, img.transform, sig.constant, sig.speed, sig.timer | 5/10 | 43 s |  | 5/5 (100%) |
| kaleido | **invalid** | 12 | img.kaleidoscope, img.noise, sig.noise, sig.speed, sig.timecode_in | 56/8 | 56 s | 1 node error(s): time1: no MIDI input ports | 2/3 (67%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/4 | 11 s |  | 5/5 (100%) |
| edges | valid | 5 | cmp.window, img.blend, img.edge, img.level, img.noise | 3/7 | 31 s |  | 3/3 (100%) |
| mirror-tile | valid | 4 | img.mirror, img.ramp, img.tile | 0/4 | 13 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, img.ramp, sig.pattern | 0/4 | 15 s |  | 4/4 (100%) |
| threshold | valid | 4 | img.blur, img.noise, img.threshold | 0/4 | 26 s |  | 4/4 (100%) |
| shape-spin | valid | 4 | img.render, img.shape, pnt.scatter, pnt.twist | 9/8 | 61 s |  | 4/4 (100%) |
| bloom-glow | valid | 6 | img.blend, img.bloom, img.level, img.noise, img.shape | 43/8 | 60 s |  | 3/3 (100%) |
| displace | valid | 3 | img.displace, img.noise, img.ramp | 0/4 | 13 s |  | 4/4 (100%) |
| particles | valid | 3 | cmp.window, img.render_points, pnt.particles | 2/5 | 25 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/4 | 12 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/4 | 15 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/4 | 18 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 0/4 | 14 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 9 | img.layer, img.mixer, img.noise | 1/5 | 26 s | missing img.composite | 2/3 (67%) |
| reaction-diffusion | valid | 5 | img.blend, img.level, img.noise, img.reaction_diffusion | 7/8 | 33 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.blend, img.noise, img.text | 0/4 | 11 s |  | 4/4 (100%) |
