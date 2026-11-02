# Network-generation eval

**Model:** `ollama:qwen2.5-coder:7b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 9/20 (**45%**) · **Score:** 56/78 points (**73%** mean per prompt) · **Time:** 2931 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 12 | img.blur, img.noise | 0/40 | 33 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 6 | sig.beat, sig.constant, sig.lfo, sig.osc_in, sig.osc_out | 11/40 | 28 s | missing img.ramp; missing img.hsv_adjust; nothing is animated | 2/5 (40%) |
| feedback-trails | **invalid** | 2 | img.feedback, img.noise | 35/111 | 49 s | 2 nodes < 4; missing img.transform; missing img.composite | 2/5 (40%) |
| kaleido | valid | 8 | img.kaleidoscope, img.noise | 0/40 | 25 s |  | 3/3 (100%) |
| lfo-brightness | **invalid** | 3 | img.blend, img.constant, sig.lfo | 1/49 | 31 s | missing img.level; nothing is animated | 3/5 (60%) |
| edges | valid | 10 | img.constant, img.edge | 0/40 | 37 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 3 | img.mirror | 17/40 | 30 s | missing img.tile | 3/4 (75%) |
| two-sources-blend | valid | 6 | img.blend, img.noise | 1/40 | 38 s |  | 4/4 (100%) |
| threshold | valid | 4 | img.blur, img.threshold | 0/40 | 23 s |  | 4/4 (100%) |
| shape-spin | **invalid** | 1 | msh.circle | 1/40 | 216 s | missing img.shape; nothing is animated | 2/4 (50%) |
| bloom-glow | valid | 4 | img.bloom, img.blur, img.level | 1/40 | 25 s |  | 3/3 (100%) |
| displace | **invalid** | 2 | img.displace, img.noise | 2/40 | 26 s | 2 nodes < 3 | 3/4 (75%) |
| particles | valid | 5 | img.render, img.render_points, pnt.age, pnt.attribute, pnt.points | 7/40 | 31 s |  | 4/4 (100%) |
| mesh-sphere | **invalid** | 0 |  | 0/40 | 83 s | 0 nodes < 2; missing msh.; missing img.render | 1/4 (25%) |
| math-channels | **invalid** | 3 | img.noise, img.transform, sig.lfo | 5/397 | 1933 s | 3 nodes < 4; missing sig.math | 4/6 (67%) |
| colour-grade | valid | 3 | img.level, img.noise | 0/40 | 25 s |  | 3/3 (100%) |
| lookup-recolour | **invalid** | 0 |  | 0/40 | 20 s | 0 nodes < 3; missing img.lookup | 1/3 (33%) |
| composite-layers | **invalid** | 3 | img.noise | 30/40 | 38 s | 3 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | **invalid** | 1 | img.reaction_diffusion | 80/202 | 209 s | 1 nodes < 2 | 2/3 (67%) |
| text-overlay | valid | 2 | img.noise, img.text | 18/40 | 33 s |  | 4/4 (100%) |
