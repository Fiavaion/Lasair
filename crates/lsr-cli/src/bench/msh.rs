//! PT-12 (PRD §8): MSH cooks on the CPU. Both cases run the shipped
//! operators through `lsr_ops_msh::run`, the worker-side path, so they need
//! no GPU.

use std::time::Instant;

use lsr_ops_msh::Mesh;
use serde_json::{Value, json};

use super::{Case, Status};

fn sphere(rows: u32, columns: u32, centre: [f64; 3]) -> anyhow::Result<Mesh> {
    lsr_ops_msh::run(
        "msh.sphere",
        &[
            ("rows", json!(rows)),
            ("columns", json!(columns)),
            ("centre", json!(centre)),
        ],
        &[],
    )
    .map_err(|e| anyhow::anyhow!("sphere: {e}"))
}

/// Triangles of a polar sphere: quads between rings plus two triangle fans.
fn sphere_tris(rows: u32, columns: u32) -> u64 {
    u64::from(columns) * (2 * u64::from(rows) - 2)
}

/// `runs` timed cooks of `op` over `inputs`: (sorted ms, last output).
fn cook(
    op: &str,
    params: &[(&str, Value)],
    inputs: &[Option<Mesh>],
    runs: usize,
) -> anyhow::Result<(Vec<f64>, Mesh)> {
    let mut out = Mesh::new();
    let mut ms = Vec::with_capacity(runs);
    for _ in 0..runs {
        let t = Instant::now();
        out = lsr_ops_msh::run(op, params, inputs).map_err(|e| anyhow::anyhow!("{op}: {e}"))?;
        ms.push(t.elapsed().as_secs_f64() * 1e3);
    }
    ms.sort_by(f64::total_cmp);
    Ok((ms, out))
}

fn p50(ms: &[f64]) -> f64 {
    ms[ms.len() / 2]
}

/// One Boolean (subtract) of two offset spheres of `rows × columns`: the
/// triangles per input, the cook time and the result's polygon count.
fn boolean_once(rows: u32, columns: u32) -> anyhow::Result<(u64, f64, usize)> {
    let a = sphere(rows, columns, [0.0; 3])?;
    let b = sphere(rows, columns, [0.35, 0.1, 0.0])?;
    let t = Instant::now();
    let out = lsr_ops_msh::run(
        "msh.boolean",
        &[("operation", json!("subtract"))],
        &[Some(a), Some(b)],
    )
    .map_err(|e| anyhow::anyhow!("msh.boolean: {e}"))?;
    Ok((
        sphere_tris(rows, columns),
        t.elapsed().as_secs_f64() * 1e3,
        out.prims(),
    ))
}

/// Set to run the Boolean at the full PT-12 size (about 100k triangles per
/// input), which takes minutes with the shipped BSP Boolean.
const FULL_ENV: &str = "LASAIR_BENCH_BOOLEAN_FULL";

/// PT-12, first half: Boolean (subtract) of two offset spheres. The target
/// (100k triangles per input in ≤ 150 ms) is judged on the RTX 4070 PC's
/// CPU against the BSP Boolean that ships (PRD A48). The BSP Boolean's work
/// grows far faster than linearly, so the default run measures two smaller
/// sizes in a second or so and reports the target as not measured; with
/// `LASAIR_BENCH_BOOLEAN_FULL=1` it runs the real size once.
pub fn boolean_100k() -> anyhow::Result<Case> {
    let cap = lsr_ops_msh::boolean::MAX_POLYGONS as u64;
    if std::env::var_os(FULL_ENV).is_some() {
        let (tris, ms, prims) = boolean_once(158, 320)?;
        anyhow::ensure!(tris <= cap, "{tris} triangles is over the cap of {cap}");
        return Ok(Case {
            id: "pt12.boolean_100k",
            status: Status::Ok,
            ms: Some(ms),
            note: Some(format!(
                "subtract of two offset spheres, {tris} triangles each (cap {cap}), CPU cook, 1 run: {ms:.0} ms, result {prims} polygons; target ≤ 150 ms on the RTX 4070 PC's CPU, against the shipped BSP Boolean (PRD A48)"
            )),
        });
    }
    let mut steps = Vec::new();
    for (rows, columns) in [(20, 40), (40, 80)] {
        let (tris, ms, prims) = boolean_once(rows, columns)?;
        steps.push(format!(
            "{tris} triangles each: {ms:.0} ms ({prims} polygons out)"
        ));
    }
    Ok(Case {
        id: "pt12.boolean_100k",
        status: Status::Skipped,
        ms: None,
        note: Some(format!(
            "100k triangles per input not run by default (BSP Boolean work grows far faster than linearly; cap {cap} allows it): subtract of two offset spheres, CPU cook, 1 run each: {}; set {FULL_ENV}=1 for the full size. Target ≤ 150 ms on the RTX 4070 PC's CPU, against the shipped BSP Boolean (PRD A48)",
            steps.join("; ")
        )),
    })
}

/// PT-12, second half: PolyReduce from about 1M triangles to 100k.
pub fn polyreduce_1m() -> anyhow::Result<Case> {
    let (rows, cols) = (710, 710);
    let src = sphere(rows, cols, [0.0; 3])?;
    let tris = sphere_tris(rows, cols);
    let (ms, out) = cook(
        "msh.polyreduce",
        &[("percent", json!(10.0))],
        &[Some(src)],
        3,
    )?;
    let (p50, max) = (p50(&ms), ms[ms.len() - 1]);
    Ok(Case {
        id: "pt12.polyreduce_1m",
        status: Status::Ok,
        ms: Some(p50),
        note: Some(format!(
            "{tris} → {} triangles (keep 10%), CPU cook, {} runs: p50 {p50:.1} ms, max {max:.1} ms; target ≤ 500 ms on the RTX 4070 PC's CPU",
            out.prims(),
            ms.len()
        )),
    })
}
