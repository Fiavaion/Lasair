# Network-generation eval

**Model:** `openai:Qwen3.6-35B-A3B-NVFP4` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 14/20 (**70%**) · **Score:** 70/78 points (**89%** mean per prompt) · **Time:** 1064 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/6 | 8 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 2 | img.compute, img.hsv_adjust | 1/15 | 34 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 5 | img.blur, img.composite, img.feedback, img.level, img.noise | 0/9 | 19 s | missing img.transform | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/8 | 11 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 0/8 | 10 s |  | 5/5 (100%) |
| edges | valid | 3 | img.edge, img.level, img.noise | 0/7 | 10 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.ramp, img.tile | 2/10 | 11 s |  | 4/4 (100%) |
| two-sources-blend | valid | 5 | img.blend, img.channel_image, img.compute, img.noise, sig.lfo | 3/23 | 40 s |  | 4/4 (100%) |
| threshold | **invalid** | 3 | img.blur, img.level, img.noise | 0/6 | 7 s | missing img.threshold | 3/4 (75%) |
| shape-spin | **invalid** | 2 | img.compute, img.constant | 2/11 | 17 s | missing img.shape; nothing is animated | 2/4 (50%) |
| bloom-glow | valid | 7 | img.bloom, img.composite, img.constant, img.shape | 1/11 | 30 s |  | 3/3 (100%) |
| displace | valid | 5 | img.channel_image, img.constant, img.displace, img.noise, sig.lfo | 2/47 | 93 s |  | 4/4 (100%) |
| particles | valid | 8 | img.render_points, pnt.age, pnt.attract, pnt.colour, pnt.emitter, pnt.integrate, pnt.points, pnt.solver | 2/15 | 27 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.iso_surface | 0/13 | 19 s |  | 4/4 (100%) |
| math-channels | valid | 4 | img.noise, img.transform, sig.lfo, sig.math | 0/9 | 12 s |  | 6/6 (100%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/7 | 9 s |  | 3/3 (100%) |
| lookup-recolour | valid | 4 | img.channel_image, img.lookup, img.noise, sig.lfo | 0/10 | 14 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 4 | img.constant, img.mixer, img.noise | 11/30 | 30 s | 4 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | **invalid** | 5 | img.bloom, img.compute, img.feedback, img.level | 0/40 | 651 s | 1 node error(s): rd_sim: line 32, col 16: expected variable access, found "p1" | 2/3 (67%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 0/8 | 11 s |  | 4/4 (100%) |
