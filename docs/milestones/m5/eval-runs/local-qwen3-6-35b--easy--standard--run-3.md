# Network-generation eval

**Model:** `ollama:qwen3.6:35b` · **Assist:** standard · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts.toml` · **Valid:** 18/20 (**90%**) · **Score:** 76/78 points (**98%** mean per prompt) · **Time:** 119 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| noise-blur | valid | 2 | img.blur, img.noise | 0/1 | 10 s |  | 4/4 (100%) |
| gradient-colour | valid | 3 | img.hsv_adjust, img.ramp, sig.lfo | 0/1 | 4 s |  | 5/5 (100%) |
| feedback-trails | **invalid** | 4 | img.blend, img.feedback, img.noise, img.transform | 0/1 | 6 s | missing img.composite | 4/5 (80%) |
| kaleido | valid | 2 | img.kaleidoscope, img.noise | 0/1 | 4 s |  | 3/3 (100%) |
| lfo-brightness | valid | 3 | img.constant, img.level, sig.lfo | 1/2 | 4 s |  | 5/5 (100%) |
| edges | valid | 3 | cmp.window, img.edge, img.noise | 0/1 | 4 s |  | 3/3 (100%) |
| mirror-tile | valid | 3 | img.mirror, img.ramp, img.tile | 0/1 | 5 s |  | 4/4 (100%) |
| two-sources-blend | valid | 4 | img.blend, img.noise, img.ramp, sig.lfo | 0/1 | 5 s |  | 4/4 (100%) |
| threshold | valid | 3 | img.blur, img.noise, img.threshold | 0/1 | 5 s |  | 4/4 (100%) |
| shape-spin | valid | 2 | img.shape, img.transform | 0/1 | 4 s |  | 4/4 (100%) |
| bloom-glow | valid | 4 | img.bloom, img.mixer, img.noise, img.shape | 9/4 | 12 s |  | 3/3 (100%) |
| displace | valid | 4 | cmp.window, img.displace, img.noise, img.ramp | 0/1 | 5 s |  | 4/4 (100%) |
| particles | valid | 2 | img.render_points, pnt.particles | 0/1 | 6 s |  | 4/4 (100%) |
| mesh-sphere | valid | 2 | img.render, msh.sphere | 0/1 | 4 s |  | 4/4 (100%) |
| math-channels | **invalid** | 4 | img.noise, img.transform, sig.lfo, sig.math | 5/4 | 13 s | nothing is animated | 5/6 (83%) |
| colour-grade | valid | 3 | img.hsv_adjust, img.level, img.noise | 0/1 | 5 s |  | 3/3 (100%) |
| lookup-recolour | valid | 3 | img.lookup, img.noise, img.ramp | 0/1 | 4 s |  | 3/3 (100%) |
| composite-layers | valid | 6 | img.blend, img.composite, img.constant, img.noise, img.ramp | 2/2 | 10 s |  | 3/3 (100%) |
| reaction-diffusion | valid | 4 | img.blend, img.noise, img.ramp, img.reaction_diffusion | 1/2 | 6 s |  | 3/3 (100%) |
| text-overlay | valid | 3 | img.composite, img.noise, img.text | 0/1 | 5 s |  | 4/4 (100%) |
