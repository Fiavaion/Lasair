# Network-generation eval

**Model:** `ollama:gemma4:12b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 7/20 (**35%**) · **Score:** 57/78 points (**74%** mean per prompt) · **Time:** 1632 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 2/28 | 114 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 3 | img.constant, img.hsv_adjust, sig.lfo | 0/40 | 142 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 0 |  | 0/4 | 13 s | 0 nodes < 4; missing img.feedback; missing img.transform; missing img.composite | 1/5 (20%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/10 | 19 s |  | 3/3 (100%) |
| lfo-brightness | valid | 6 | img.constant, img.level, sig.constant, sig.lfo, sig.math | 1/13 | 41 s |  | 5/5 (100%) |
| edges | valid | 2 | img.edge, img.noise | 0/8 | 21 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 5 | img.constant, img.kernel, img.mirror | 0/11 | 55 s | missing img.tile | 3/4 (75%) |
| two-sources-blend | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/22 | 141 s | missing img.blend; nothing is animated | 2/4 (50%) |
| threshold | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/27 | 154 s | missing img.threshold | 3/4 (75%) |
| shape-spin | **invalid** | 2 | img.constant, img.kernel | 0/11 | 39 s | missing img.shape; nothing is animated | 2/4 (50%) |
| bloom-glow | valid | 2 | img.bloom, img.kernel | 0/6 | 17 s |  | 3/3 (100%) |
| displace | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/20 | 139 s | missing img.displace | 3/4 (75%) |
| particles | **invalid** | 6 | img.constant, img.kernel, pnt.age, pnt.attribute_noise, pnt.drag, pnt.emitter | 0/12 | 40 s | missing img.render/img.render_points | 3/4 (75%) |
| mesh-sphere | **invalid** | 6 | cmp.window, img.blur, img.composite, img.constant, img.noise, msh.iso_surface | 0/30 | 389 s | missing img.render | 3/4 (75%) |
| math-channels | **invalid** | 3 | img.noise, sig.lfo, sig.math | 0/8 | 22 s | 3 nodes < 4; missing img.transform | 4/6 (67%) |
| colour-grade | valid | 4 | img.hsv_adjust, img.level, img.noise | 0/13 | 42 s |  | 3/3 (100%) |
| lookup-recolour | **invalid** | 0 |  | 0/5 | 52 s | 0 nodes < 3; missing img.lookup | 1/3 (33%) |
| composite-layers | valid | 5 | img.composite, img.fluid, img.kernel, img.nca | 0/11 | 39 s |  | 3/3 (100%) |
| reaction-diffusion | **invalid** | 0 |  | 0/2 | 17 s | 0 nodes < 2; missing img. | 1/3 (33%) |
| text-overlay | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/18 | 137 s | missing img.text | 3/4 (75%) |
