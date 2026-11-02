# Network-generation eval

**Model:** `ollama:qwen3.6:35b` · **Assist:** full · **Retrieval:** semantic · **Prompts:** `docs/milestones/m5/eval-prompts-hard.toml` · **Valid:** 6/15 (**40%**) · **Score:** 191/233 points (**83%** mean per prompt) · **Time:** 347 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| h-tunnel-two-screens | **invalid** | 11 | cmp.window, img.bloom, img.kaleidoscope, img.kernel, img.mixer, img.noise | 2/13 | 59 s | missing img.feedback; missing img.transform; missing img.composite; missing img.hsv_adjust; no path img.noise > img.kaleidoscope > img.composite > cmp.window; no path img.noise > img.hsv_adjust > cmp.window; no path img.feedback > img.transform > img.composite > img.feedback; no path img.feedback > img.level/img.hsv_adjust > img.feedback; no img.noise.octaves = 4; no img.noise.monochrome = false; img.hsv_adjust hue not driven as asked; not met in any accepted way: the trails fade (a Level brightness or an HSV Adjust value below 1); 1 node error(s): colour_cycle: kernel:4:12: error: unknown function `hsv2rgb`
  @Cd = vec4(hsv2rgb(hsv), 1.0);
             ^^^^^^^
; nothing is animated | 9/23 (39%) |
| h-layered-stage | **invalid** | 10 | cmp.window, img.blur, img.composite, img.mirror, img.noise, img.ramp, img.shape, sig.lfo | 5/7 | 44 s | missing img.tile; no path img.shape > img.tile; no img.tile.repeat = [3,3]; not met in any accepted way: the three layers combined (two composites: add then multiply, or one mixer: add on layer 1, multiply on layer 2) | 13/17 (76%) |
| h-three-looks | **invalid** | 9 | cmp.window, img.mixer, img.noise, img.ramp, img.shape, sig.lfo, sig.speed, sig.timer | 11/9 | 34 s | missing img.switch; no path img.noise > img.switch > cmp.window; no path img.shape > img.switch; no path img.ramp > img.switch; no img.switch.blend = true; img.switch index every 12 s not driven as asked; 0 x img.switch (want 1..1) | 10/17 (59%) |
| h-torus-dust | **invalid** | 6 | img.render, mat.point_sprite, msh.torus, pnt.colour, pnt.scatter, pnt.turbulence | 12/7 | 32 s | missing msh.scatter; no path msh.torus > msh.scatter > pnt.noise/pnt.turbulence > img.render; no path msh.scatter > pnt.colour > img.render; no msh.scatter.count = 5000 | 13/17 (76%) |
| h-spark-fountain | **invalid** | 3 | img.bloom, img.render_points, pnt.particles | 0/4 | 11 s | no pnt.particles.life = 2; no pnt.particles.rate >= 2700 <= 3300; no pnt.particles.velocity >= 0.1 (component 1) | 10/13 (77%) |
| h-orbit-cam | valid | 5 | img.render, mat.toon, mat.unlit, msh.sphere, msh.torus | 0/4 | 11 s |  | 19/19 (100%) |
| h-breathing-clouds | **invalid** | 8 | img.hsv_adjust, img.noise, img.share_out, img.transform, sig.constant, sig.lfo, sig.math | 2/5 | 16 s | missing sig.noise; img.transform scale bound from sig.lfo over 0.5..2 every 4 s not driven as asked; img.hsv_adjust hue bound from sig.noise not driven as asked | 7/10 (70%) |
| h-cue-sheet | valid | 4 | dat.table, img.level, img.noise, img.transform | 0/4 | 13 s |  | 17/17 (100%) |
| h-worn-tape | **invalid** | 11 | img.blend, img.blur, img.composite, img.level, img.noise, img.ramp, img.share_out, sig.lfo, sig.speed | 4/6 | 30 s | missing img.feedback; no path img.feedback > img.composite/img.blend/img.mixer | 7/9 (78%) |
| h-stark-poster | **invalid** | 6 | img.composite, img.level, img.mirror, img.noise, img.ramp, img.threshold | 0/4 | 12 s | missing img.lookup; no path img.noise > img.threshold > img.lookup; no path img.ramp > img.lookup | 10/13 (77%) |
| h-fix-broken-chain | valid | 7 | cmp.window, img.bloom, img.blur, img.noise, img.resolution, img.transform, sig.lfo | 0/6 | 15 s |  | 14/14 (100%) |
| h-share-one-source | valid | 8 | cmp.window, img.blur, img.composite, img.hsv_adjust, img.level, img.noise | 0/4 | 15 s |  | 16/16 (100%) |
| h-fix-trails | **invalid** | 6 | cmp.window, img.composite, img.feedback, img.level, img.noise, img.transform | 6/14 | 30 s | no path img.feedback > img.transform > img.composite > img.feedback; no path img.feedback > img.level > img.composite > img.feedback | 14/16 (88%) |
| h-show-change-request | valid | 17 | cmp.window, img.blend, img.bloom, img.blur, img.composite, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.shape, img.text, img.transform, sig.lfo, sig.noise | 0/2 | 17 s |  | 20/20 (100%) |
| h-wire-the-signals | valid | 6 | cmp.window, img.hsv_adjust, img.level, img.noise, sig.lfo, sig.noise | 0/2 | 8 s |  | 12/12 (100%) |
