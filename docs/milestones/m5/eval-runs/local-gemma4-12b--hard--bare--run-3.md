# Network-generation eval

**Model:** `ollama:gemma4:12b` · **Assist:** bare · **Retrieval:** none (only `--assist full` retrieves) · **Prompts:** `docs/milestones/m5/eval-prompts-hard.toml` · **Valid:** 2/15 (**13%**) · **Score:** 117/233 points (**51%** mean per prompt) · **Time:** 2415 s · **Max turns:** 40

Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.

| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |
|---|---|---|---|---|---|---|---|
| h-tunnel-two-screens | **invalid** | 9 | img.bloom, img.constant, img.displace, img.feedback, img.hsv_adjust, img.kaleidoscope, img.noise | 0/11 | 44 s | 9 nodes < 10; missing img.transform; missing img.composite; missing cmp.window; no path img.noise > img.kaleidoscope > img.composite > cmp.window; no path img.noise > img.hsv_adjust > cmp.window; no path img.noise > img.bloom > cmp.window; no path img.feedback > img.transform > img.composite > img.feedback; no path img.feedback > img.level/img.hsv_adjust > img.feedback; no img.noise.monochrome = false; 0 x cmp.window (want 2..2); not met in any accepted way: the trails fade (a Level brightness or an HSV Adjust value below 1) | 11/23 (48%) |
| h-layered-stage | **invalid** | 5 | img.composite, img.kernel | 0/9 | 42 s | 5 nodes < 6; missing img.ramp; missing img.shape; missing img.tile; missing img.noise; missing cmp.window; no path img.shape > img.tile; no img.ramp.type = "radial"; no img.shape.shape = "polygon"; no img.shape.sides = 5; no img.tile.repeat = [3,3]; no img.noise.scale <= 3; 0 x cmp.window (want 1..1); not met in any accepted way: the three layers combined (two composites: add then multiply, or one mixer: add on layer 1, multiply on layer 2); nothing is animated | 2/17 (12%) |
| h-three-looks | **invalid** | 12 | img.blend, img.constant, img.kernel, sig.constant, sig.lfo | 0/34 | 597 s | missing img.noise; missing img.shape; missing img.ramp; missing img.switch; missing cmp.window; no path img.noise > img.switch > cmp.window; no path img.shape > img.switch; no path img.ramp > img.switch; no img.shape.sides = 6; no img.switch.blend = true; no img.ramp.type = "radial"; img.switch index every 12 s not driven as asked; 0 x img.switch (want 1..1); not met in any accepted way: the shape spins (Shape rotate, or a Transform rotate on its chain); 1 node error(s): shape: kernel:3:1: error: `while` loops are not supported
  while(d < 0) d += 6.283185;
  ^^^^^
  hint: use a counted loop: for (int i = 0; i < n; i++)

kernel:4:1: error: `while` loops are not supported
  while(d > 6.283185) d -= 6.283185;
  ^^^^^
  hint: use a counted loop: for (int i = 0; i < n; i++)
; nothing is animated | 1/17 (6%) |
| h-torus-dust | **invalid** | 0 |  | 0/7 | 28 s | 0 nodes < 6; missing msh.torus; missing msh.scatter; missing pnt.noise/pnt.turbulence; missing pnt.colour; missing mat.point_sprite; missing img.render; no path msh.torus > msh.scatter > pnt.noise/pnt.turbulence > img.render; no path msh.scatter > pnt.colour > img.render; no path mat.point_sprite > img.render; no msh.scatter.count = 5000; no pnt.colour.attribute = "P"; no pnt.colour.measure = "y"; pnt.noise/pnt.turbulence evolution moving with time not driven as asked; img.render azimuth moving with time not driven as asked; nothing is animated | 1/17 (6%) |
| h-spark-fountain | **invalid** | 10 | img.bloom, img.blur, img.composite, img.constant, msh.sphere, pnt.age, pnt.emitter, pnt.gravity, pnt.points | 1/38 | 353 s | missing pnt.particles; missing img.render_points; no path pnt.particles > img.render_points > img.bloom; no pnt.particles.life = 2; no pnt.particles.rate >= 2700 <= 3300; no pnt.particles.radius <= 0.3; no pnt.particles.emitter = "sphere"; no pnt.particles.velocity >= 0.1 (component 1); no pnt.particles.gravity <= -0.1 (component 1); no img.render_points.blend = "add" | 3/13 (23%) |
| h-orbit-cam | **invalid** | 0 |  | 1/9 | 60 s | 0 nodes < 5; missing msh.sphere; missing msh.torus; missing mat.toon; missing mat.unlit; missing img.render; no path msh.sphere > img.render; no path msh.torus > img.render; no path mat.toon > img.render; no path mat.unlit > img.render; no msh.sphere.radius = 0.5; no msh.torus.radius = 1; no msh.torus.thickness in [0.05,0.1]; no img.render.elevation = 25; no img.render.distance = 5; img.render azimuth moving with time not driven as asked; 0 x img.render (want 1..1); nothing is animated | 1/19 (5%) |
| h-breathing-clouds | **invalid** | 6 | img.composite, img.constant, img.kernel, sig.lfo, sig.noise | 0/40 | 459 s | missing img.noise; missing img.transform; missing img.hsv_adjust; img.transform scale bound from sig.lfo over 0.5..2 every 4 s not driven as asked; img.hsv_adjust hue bound from sig.noise not driven as asked; 2 node error(s): clouds: kernel:2:18: error: unexpected character `\\`
  float speed = ch(\"speed\", 1.0, 0.0, 10.0);
                   ^

kernel:2:19: error: unterminated string
  float speed = ch(\"speed\", 1.0, 0.0, 10.0);
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^

kernel:3:1: error: expected `,` or `)`, found `float`
  float time = @Time * speed;
  ^^^^^
; nothing is animated | 3/10 (30%) |
| h-cue-sheet | **invalid** | 5 | dat.select, dat.table, img.hsv_adjust, img.level, img.noise | 0/40 | 335 s | missing img.transform; img.level brightness from dat.table not driven as asked; img.transform rotate from dat.table not driven as asked; 1 node error(s): mid_row_select: no column `brightness` | 13/17 (76%) |
| h-worn-tape | **invalid** | 4 | img.blur, img.composite, img.constant, img.noise | 0/20 | 139 s | 4 nodes < 5; missing img.feedback; no path img.feedback > img.composite/img.blend/img.mixer; not met in any accepted way: the brightness flickers (a Level brightness or an HSV Adjust value is driven); nothing is animated | 4/9 (44%) |
| h-stark-poster | **invalid** | 5 | img.constant, img.level, img.lookup, img.mirror, img.noise | 0/40 | 269 s | missing img.threshold; missing img.ramp; no path img.noise > img.threshold > img.lookup; no path img.ramp > img.lookup; no img.threshold.softness <= 0.05 | 8/13 (62%) |
| h-fix-broken-chain | **invalid** | 7 | cmp.window, img.bloom, img.blur, img.noise, img.resolution, img.transform, sig.lfo | 35/40 | 46 s | no path img.noise > img.blur > cmp.window; not met in any accepted way: full HD (pixels 1920 x 1080, or fraction 1.5 of the 1280 x 720 project); 1 node error(s): shake: expression for `rotate`: op('lfo_main'): no such node | 11/14 (79%) |
| h-share-one-source | valid | 8 | cmp.window, img.blur, img.composite, img.hsv_adjust, img.level, img.noise | 0/4 | 7 s |  | 16/16 (100%) |
| h-fix-trails | **invalid** | 6 | cmp.window, img.composite, img.feedback, img.level, img.noise, img.transform | 1/10 | 17 s | no path img.feedback > img.transform > img.composite > img.feedback; no path img.feedback > img.level > img.composite > img.feedback; no img.composite.operation in ["add","screen","maximum"] | 13/16 (81%) |
| h-show-change-request | **invalid** | 17 | cmp.window, img.blend, img.bloom, img.blur, img.composite, img.feedback, img.hsv_adjust, img.kaleidoscope, img.level, img.noise, img.shape, img.text, img.transform, sig.lfo, sig.noise | 0/6 | 10 s | no img.text.text = "FINAL"; no cmp.window.title = "Preview" | 18/20 (90%) |
| h-wire-the-signals | valid | 6 | cmp.window, img.hsv_adjust, img.level, img.noise, sig.lfo, sig.noise | 0/6 | 9 s |  | 12/12 (100%) |
