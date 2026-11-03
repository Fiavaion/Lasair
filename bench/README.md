# Bench

`lasair bench --out bench/results/<machine>.json` runs the standard performance suite (PRD §8, FR-H4) and writes `lasair-bench/1` JSON. M0 measures start-up only (`startup.registry`, `startup.gpu_device`); each milestone adds the PT-n cases it is accountable for. Results are git-ignored; commit reference runs deliberately with `git add -f`.

The PT-12 Boolean case (`pt12.boolean_100k`) subtracts two offset spheres of about 100k triangles each with the shipped Manifold Boolean (PRD A74), 5 runs, and reports p50 and max against the ≤ 150 ms target.
