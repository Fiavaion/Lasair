# Network-generation eval

**Model:** `ollama:gemma4:12b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 8/20 (**40%**) · **Score:** 58/78 points (**76%** mean per prompt) · **Time:** 1967 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | **invalid** | 4 | img.composite, img.constant, img.displace, img.noise | 0/28 | 226 s | missing img.blur | 3/4 (75%) |
| gradient-colour | **invalid** | 3 | img.constant, img.hsv_adjust, sig.lfo | 0/10 | 26 s | missing img.ramp | 4/5 (80%) |
| feedback-trails | **invalid** | 0 |  | 0/4 | 13 s | 0 nodes < 4; missing img.feedback; missing img.transform; missing img.composite | 1/5 (20%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/10 | 20 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 1/9 | 25 s |  | 5/5 (100%) |
| edges | valid | 2 | img.edge, img.noise | 0/8 | 21 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 6 | img.constant, img.kernel, img.mirror | 0/11 | 40 s | missing img.tile | 3/4 (75%) |
| two-sources-blend | **invalid** | 4 | img.composite, img.constant, img.displace, img.noise | 0/27 | 216 s | missing img.blend; nothing is animated | 2/4 (50%) |
| threshold | **invalid** | 3 | img.blur, img.level, img.noise | 1/20 | 68 s | missing img.threshold | 3/4 (75%) |
| shape-spin | **invalid** | 2 | img.constant, img.kernel | 0/40 | 159 s | missing img.shape; 1 node error(s): rotate: kernel:5:24: error: expected `vec4`, found `vec2`
  vec2 rotated = qrotate(p, angle);
                         ^
  hint: add components: vec4(v, 1)
; nothing is animated | 1/4 (25%) |
| bloom-glow | valid | 2 | img.bloom, img.kernel | 0/7 | 18 s |  | 3/3 (100%) |
| displace | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/21 | 142 s | missing img.displace | 3/4 (75%) |
| particles | valid | 7 | img.composite, img.constant, img.render_points, pnt.age, pnt.attribute_noise, pnt.drag, pnt.emitter | 0/15 | 63 s |  | 4/4 (100%) |
| mesh-sphere | **invalid** | 7 | cmp.window, img.blur, img.composite, img.constant, img.kernel, img.noise, msh.iso_surface | 0/27 | 452 s | missing img.render | 3/4 (75%) |
| math-channels | **invalid** | 3 | img.noise, sig.lfo, sig.math | 2/40 | 137 s | 3 nodes < 4; missing img.transform | 4/6 (67%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/9 | 26 s |  | 3/3 (100%) |
| lookup-recolour | valid | 17 | img.constant, img.kernel, img.lookup, img.noise | 0/11 | 82 s |  | 3/3 (100%) |
| composite-layers | valid | 5 | img.composite, img.fluid, img.kernel, img.nca | 0/11 | 41 s |  | 3/3 (100%) |
| reaction-diffusion | **invalid** | 0 |  | 0/2 | 50 s | 0 nodes < 2; missing img. | 1/3 (33%) |
| text-overlay | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/21 | 143 s | missing img.text | 3/4 (75%) |
