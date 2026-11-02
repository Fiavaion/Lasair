# Network-generation eval

**Model:** `ollama:qwen3.6:35b` · **Assist:** full · **Retrieval:** semantic · **Prompts:** `docs/milestones/m5/eval-prompts-hard.toml` · **Valid:** 6/15 (**40%**) · **Score:** 195/233 points (**84%** mean per prompt) · **Time:** 355 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| h-tunnel-two-screens | **invalid** | 12 | cmp.window, img.bloom, img.hsv_adjust, img.kaleidoscope, img.kernel, img.mixer, img.noise, img.transform | 1/13 | 67 s | missing img.feedback; missing img.composite; no path img.noise > img.kaleidoscope > img.composite > cmp.window; no path img.feedback > img.transform > img.composite > img.feedback; no path img.feedback > img.level/img.hsv_adjust > img.feedback; no img.noise.octaves = 4; img.hsv_adjust hue not driven as asked; not met in any accepted way: the trails fade (a Level brightness or an HSV Adjust value below 1); 1 node error(s): trail_kernel: kernel:6:35: error: expected `,` or `)`, found the end of the kernel
  @Cd = vec4(min(prev, vec3(1.0)), 1
                                    ^
 | 14/23 (61%) |
| h-layered-stage | **invalid** | 8 | cmp.window, img.blend, img.composite, img.mirror, img.noise, img.ramp, img.shape | 7/9 | 43 s | missing img.tile; no path img.shape > img.tile; no img.tile.repeat = [3,3]; no img.noise.scale <= 3; not met in any accepted way: the three layers combined (two composites: add then multiply, or one mixer: add on layer 1, multiply on layer 2) | 12/17 (71%) |
| h-three-looks | **invalid** | 10 | cmp.window, img.level, img.mixer, img.noise, img.ramp, img.switch, sig.lfo, sig.speed, sig.timer | 7/8 | 34 s | missing img.shape; no path img.noise > img.switch > cmp.window; no path img.shape > img.switch; no path img.ramp > img.switch; no img.shape.sides = 6; no img.switch.blend = true; not met in any accepted way: the shape spins (Shape rotate, or a Transform rotate on its chain) | 10/17 (59%) |
| h-torus-dust | **invalid** | 6 | img.render, img.render_points, msh.torus, pnt.colour, pnt.scatter, pnt.turbulence | 18/8 | 37 s | missing msh.scatter; missing mat.point_sprite; no path msh.torus > msh.scatter > pnt.noise/pnt.turbulence > img.render; no path msh.scatter > pnt.colour > img.render; no path mat.point_sprite > img.render; no msh.scatter.count = 5000 | 11/17 (65%) |
| h-spark-fountain | **invalid** | 3 | img.bloom, img.render_points, pnt.particles | 1/5 | 17 s | no pnt.particles.velocity >= 0.1 (component 1) | 12/13 (92%) |
| h-orbit-cam | valid | 5 | img.render, mat.toon, mat.unlit, msh.sphere, msh.torus | 0/4 | 11 s |  | 19/19 (100%) |
| h-breathing-clouds | **invalid** | 11 | img.hsv_adjust, img.level, img.noise, img.transform, sig.constant, sig.lfo, sig.math | 3/5 | 19 s | missing sig.noise; img.transform scale bound from sig.lfo over 0.5..2 every 4 s not driven as asked; img.hsv_adjust hue bound from sig.noise not driven as asked | 7/10 (70%) |
| h-cue-sheet | valid | 4 | dat.table, img.level, img.noise, img.transform | 0/4 | 12 s |  | 17/17 (100%) |
| h-worn-tape | **invalid** | 7 | img.blend, img.blur, img.level, img.noise, sig.lfo, sig.speed | 5/6 | 18 s | missing img.feedback; no path img.feedback > img.composite/img.blend/img.mixer | 7/9 (78%) |
| h-stark-poster | **invalid** | 5 | img.composite, img.mirror, img.noise, img.ramp, img.threshold | 0/4 | 11 s | missing img.lookup; no path img.noise > img.threshold > img.lookup; no path img.ramp > img.lookup | 10/13 (77%) |
| h-fix-broken-chain | valid | 7 | cmp.window, img.bloom, img.blur, img.noise, img.resolution, img.transform, sig.lfo | 0/6 | 14 s |  | 14/14 (100%) |
| h-share-one-source | valid | 8 | cmp.window, img.blur, img.composite, img.hsv_adjust, img.level, img.noise | 0/4 | 12 s |  | 16/16 (100%) |
| h-fix-trails | **invalid** | 6 | cmp.window, img.composite, img.feedback, img.level, img.noise, img.transform | 6/11 | 26 s | no path img.feedback > img.transform > img.composite > img.feedback; no path img.feedback > img.level > img.composite > img.feedback | 14/16 (88%) |
| h-show-change-request | valid | 17 | cmp.window, img.blend, img.bloom, img.blur, img.composite, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.shape, img.text, img.transform, sig.lfo, sig.noise | 0/4 | 26 s |  | 20/20 (100%) |
| h-wire-the-signals | valid | 6 | cmp.window, img.hsv_adjust, img.level, img.noise, sig.lfo, sig.noise | 0/2 | 7 s |  | 12/12 (100%) |
