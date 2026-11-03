# PT targets (M6 AC1), measured 2026-11-03

`lasair bench` on the Apple M5 Pro (48 GB, Metal), quiet machine (Ollama
idle, no builds), commit 07b5dd8. Times include GPU completion. Most PT
targets name an RTX machine; on this Mac those rows are reference numbers,
and the verdict waits for the hand check on that machine.

| PT | Target (machine) | This Mac | Verdict |
|---|---|---|---|
| PT-1 | 4K60, 20-node IMG chain (RTX 4070) | `pt1.img_chain_4k` p50 20.6 ms, p95 21.1 ms (48 fps) | RTX hand check |
| PT-2 | 2M particles at 1080p60 (RTX 4070) | `pt2.particles_2m` p50 9.9 ms, p95 11.1 ms (101 fps) | RTX hand check (met here) |
| PT-3 / PT-4 | SD-Turbo img2img ≥ 30 / ≥ 90 fps (RTX 4070 / 4090) | not measurable here (`win+nvidia`) | RTX hand check: `uv run lsr-ai-host bench --model sd-turbo` |
| PT-5 | 1080p60 compositing with 500k particles; AI interactive tier ≥ 5 fps local (M-series) | `pt5.composite_500k` p50 3.5 ms, p95 3.9 ms (290 fps); AI tier: SD-Turbo 512² through the AI Host protocol 13.1 fps at 1 step, 8.7 fps at 2 steps | **met** (both halves; see the update below) |
| PT-6 | 1080p30, AI off (GTX 1660 / M1) | no M1 here; this Mac is far above it (PT-5 row) | M1 / GTX 1660 hand check |
| PT-7 | clip launch ≤ 1 frame HAP, ≤ 3 frames H.264/H.265 (RTX 4070) | `pt7.launch_cut` 1 frame at worst (cut, 8 clips) | RTX hand check (met here) |
| PT-8 | MIDI/OSC → visible change ≤ 2 frames (all) | `pt8.osc_to_pixel` 1 frame in 20 of 20 trials, median 14.3 ms | **met** |
| PT-9 | AI Host crash → frames again ≤ 5 s, output never below project fps (all) | `lsr-ops-ai` `tests/pt9.rs`: results again 135 ms after the host was killed, slowest cook 1.0 ms | **met** (engine side, test host) |
| PT-10 | 24 h soak: no leak > 100 MB, no spike > 50 ms (RTX 4070, lab image) | the 24 h run here failed on spikes while Ollama loaded the GPU; memory held | clean re-run, then the lab-image run |
| PT-11 | cold start to interactive ≤ 5 s (all) | `pt11.cold_start` slowest of 5 launches 1050 ms (first after a build), then 155–163 ms | **met** (see the update below) |
| PT-12 | Boolean of two 100k-triangle meshes ≤ 150 ms; PolyReduce 1M → 100k ≤ 500 ms (RTX 4070 PC, CPU) | Boolean: **167 s** at 100k each (one run; 50 ms at 1.5k, 0.68 s at 6k, ~10 s at 25k). PolyReduce: p50 1483 ms | superseded: see the updates below (both met here) |

## What this means for AC1

- Met here or by test: PT-5, PT-8, PT-9, PT-11, PT-12 (see the updates
  below; PT-12 still has its RTX 4070 PC hand check).
- Reference numbers only, wait on the RTX / M1 hand checks: PT-1, PT-2,
  PT-3, PT-4, PT-6, PT-7.
- Open on this side: PT-10 (clean soak).
- PT-12 Boolean is the one target the code is nowhere near. The shipped
  BSP Boolean (PRD A48, accepted for M2 with "an exact-predicate Boolean
  is M6 work") grows far faster than linearly: 100k triangles a side takes
  minutes on any CPU. Meeting 150 ms needs a different algorithm
  (exact-predicate mesh arrangements, as in Manifold, Apache-2.0); that is
  a decision for Mark. PolyReduce at 1.5 s here is also unlikely to reach
  500 ms on the 4070 PC's CPU without work.

**Update 2026-11-03 (PRD A74):** `msh.boolean` now cooks on
manifold-rust. `pt12.boolean_100k` runs the 100k size by default (5 runs):
p50 114 ms, max 136 ms on this Mac, against 167 s before. The
`LASAIR_BENCH_BOOLEAN_FULL` opt-in is gone. Verdict: RTX 4070 PC hand check
(met here).

**Update 2026-11-03 (PolyReduce):** `pt12.polyreduce_1m` p50 1483 ms →
235 ms (4-ary heap with in-place keys, flat adjacency, Z-ordered chunks on
up to 8 threads, then one exact pass; output identical for any thread
count, quality equal to the exact greedy). Verdict: RTX 4070 PC hand check
(met here). PT-12 is now met on this Mac for both cases.

**Update 2026-11-03 (PT-5 AI tier):** `uv run --extra diffusion lsr-ai-host
bench --model sd-turbo --size 512 --steps N --frames 600 --via-protocol`
(the engine's path: a `--connect` host and the shared-memory ring, 1 frame
in flight, TAESD, moving input, no similarity filter): 1 step 13.1 fps (p50
76.3 ms, p95 77.2 ms), 2 steps 8.7 fps (p50 114.4 ms, p95 115.3 ms), against
≥ 5 fps. Without TAESD (the full VAE) it drops to 4.1 / 3.6 fps, so the
interactive tier needs the tiny VAE in the model cache. Spike (e) called
PT-5 "not met" at 11 fps; that read the real-time bar (PT-3), not PT-5's
≥ 5 fps. The weights are SD-Turbo (non-commercial, accepted for benchmarks
on 2026-10-30); a commercial-safe model through the same path is still to
run. Machine: M5 Pro; PT-5 names M2 Pro or later, so an M2 Pro check would
show the margin.

**Update 2026-11-03 (PT-11):** smoke mode prints `interactive_ms` when its
second frame starts (the first is on screen and the event loop takes input),
and `pt11.cold_start` launches the `lasair-app` built beside `lasair` five
times and times spawn to that line. Here: 1050, 155, 163, 157, 158 ms (budget
5000 ms); with `examples/audio-reactive-feedback.lasair` open the in-process
time is 131–180 ms. The first launch after a build is the coldest this
machine gives without a reboot; a reboot-cold launch on the M1 and the RTX
4070 PC is part of their hand checks.
