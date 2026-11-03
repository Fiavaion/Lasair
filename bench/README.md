# Bench

`lasair bench --out bench/results/<machine>.json` runs the standard performance suite (PRD §8, FR-H4) and writes `lasair-bench/1` JSON. M0 measures start-up only (`startup.registry`, `startup.gpu_device`); each milestone adds the PT-n cases it is accountable for. Results are git-ignored; commit reference runs deliberately with `git add -f`.

The PT-12 Boolean case measures two small sizes by default, because the shipped BSP Boolean takes minutes at 100k triangles per input; set `LASAIR_BENCH_BOOLEAN_FULL=1` to run the real size once.
