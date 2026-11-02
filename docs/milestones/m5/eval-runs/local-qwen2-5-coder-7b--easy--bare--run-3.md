# Network-generation eval

**Model:** `ollama:qwen2.5-coder:7b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 7/20 (**35%**) · **Score:** 49/78 points (**66%** mean per prompt) · **Time:** 1377 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 8 | img.blur, img.noise | 0/40 | 30 s |  | 4/4 (100%) |
| gradient-colour | **invalid** | 1 | sig.lfo | 37/40 | 39 s | 1 nodes < 2; missing img.ramp; missing img.hsv_adjust; nothing is animated | 1/5 (20%) |
| feedback-trails | **invalid** | 3 | img.blend, img.feedback, img.noise | 1/50 | 31 s | 3 nodes < 4; missing img.transform; missing img.composite | 2/5 (40%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/40 | 40 s |  | 3/3 (100%) |
| lfo-brightness | **invalid** | 3 | img.blend, img.constant, sig.lfo | 1/50 | 30 s | missing img.level; nothing is animated | 3/5 (60%) |
| edges | valid | 4 | img.blend, img.constant, img.edge, img.level | 3/40 | 24 s |  | 3/3 (100%) |
| mirror-tile | **invalid** | 6 | img.blend, img.level, img.mirror | 0/40 | 34 s | missing img.tile | 3/4 (75%) |
| two-sources-blend | **invalid** | 1 | img.noise | 121/322 | 546 s | 1 nodes < 3; missing img.blend; nothing is animated | 1/4 (25%) |
| threshold | **invalid** | 1 | cmp.pack_noise_blur | 0/40 | 29 s | 1 nodes < 3; missing img.threshold; missing img.blur | 1/4 (25%) |
| shape-spin | **invalid** | 0 |  | 38/40 | 75 s | 0 nodes < 1; missing img.shape; nothing is animated | 1/4 (25%) |
| bloom-glow | valid | 4 | img.blend, img.bloom, img.level, img.noise | 22/40 | 30 s |  | 3/3 (100%) |
| displace | **invalid** | 2 | img.displace, img.noise | 2/40 | 23 s | 2 nodes < 3 | 3/4 (75%) |
| particles | valid | 3 | img.render, pnt.age | 4/40 | 41 s |  | 4/4 (100%) |
| mesh-sphere | **invalid** | 1 | msh.sphere | 10/40 | 20 s | 1 nodes < 2; missing img.render | 2/4 (50%) |
| math-channels | **invalid** | 3 | img.noise, img.transform, sig.lfo | 1/46 | 33 s | 3 nodes < 4; missing sig.math; nothing is animated | 3/6 (50%) |
| colour-grade | valid | 7 | img.level, img.noise | 0/40 | 27 s |  | 3/3 (100%) |
| lookup-recolour | valid | 7 | img.constant, img.lookup, img.noise | 14/40 | 28 s |  | 3/3 (100%) |
| composite-layers | **invalid** | 3 | img.noise | 30/40 | 40 s | 3 nodes < 5; missing img.composite | 1/3 (33%) |
| reaction-diffusion | **invalid** | 1 | img.compute | 37/40 | 63 s | 1 nodes < 2 | 2/3 (67%) |
| text-overlay | **invalid** | 4 | cmp.window, img.blend, img.noise, sig.constant | 0/40 | 194 s | missing img.text | 3/4 (75%) |
