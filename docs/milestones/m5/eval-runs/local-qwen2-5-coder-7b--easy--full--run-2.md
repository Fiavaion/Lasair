# Network-generation eval

**Model:** `ollama:qwen2.5-coder:7b` · **Assist:** full · **Retrieval:** semantic · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 17/20 (**85%**) · **Score:** 75/78 points (**96%** mean per prompt) · **Time:** 491 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/4 | 15 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 4 | img.blend, img.ramp, sig.lfo, sig.timer | 7/13 | 38 s | missing img.hsv_adjust | 4/5 (80%) |
| feedback-trails | valid | 4 | img.composite, img.feedback, img.noise, img.transform | 0/4 | 13 s |  | 5/5 (100%) |
| kaleido | **invalid** | 7 | img.blend, img.kaleidoscope, img.level, img.noise, sig.noise, sig.timecode_in | 8/11 | 30 s | 2 node error(s): time1: no MIDI input ports | 2/3 (67%) |
| lfo-brightness | valid | 4 | img.constant, img.level, sig.lfo | 18/11 | 39 s |  | 5/5 (100%) |
| edges | valid | 5 | cmp.window, img.blend, img.edge, img.level, img.noise | 3/7 | 22 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.noise, img.tile | 0/4 | 12 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, sig.lfo | 0/4 | 13 s |  | 4/4 (100%) |
| threshold | valid | 4 | img.blur, img.level, img.noise, img.threshold | 0/4 | 12 s |  | 4/4 (100%) |
| shape-spin | valid | 5 | img.shape, pnt.points, pnt.vortex, sig.math, sig.timer | 10/8 | 48 s |  | 4/4 (100%) |
| bloom-glow | valid | 7 | img.blend, img.bloom, img.composite, img.noise, img.shape | 48/8 | 68 s |  | 3/3 (100%) |
| displace | valid | 3 | img.displace, img.noise, img.ramp | 0/4 | 13 s |  | 4/4 (100%) |
| particles | valid | 2 | img.render_points, pnt.points | 1/5 | 21 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/4 | 12 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/4 | 15 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 1/5 | 15 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise | 0/4 | 10 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 12 | img.blend, img.constant, img.layer, img.noise, img.ramp | 7/8 | 47 s | missing img.composite | 2/3 (67%) |
| reaction-diffusion | valid | 5 | img.blend, img.channel_image, img.level, img.noise, img.reaction_diffusion | 8/8 | 37 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.blend, img.noise, img.text | 0/4 | 11 s |  | 4/4 (100%) |
