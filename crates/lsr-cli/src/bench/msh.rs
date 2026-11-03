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

/// PT-12, first half: Boolean (subtract) of two offset spheres of about
/// 100k triangles each, with the Manifold Boolean that ships (PRD A74). The
/// target (≤ 150 ms) is judged on the RTX 4070 PC's CPU.
pub fn boolean_100k() -> anyhow::Result<Case> {
    let (rows, cols) = (158, 320);
    let a = sphere(rows, cols, [0.0; 3])?;
    let b = sphere(rows, cols, [0.35, 0.1, 0.0])?;
    let tris = sphere_tris(rows, cols);
    let (ms, out) = cook(
        "msh.boolean",
        &[("operation", json!("subtract"))],
        &[Some(a), Some(b)],
        5,
    )?;
    let (p50, max) = (p50(&ms), ms[ms.len() - 1]);
    Ok(Case {
        id: "pt12.boolean_100k",
        status: Status::Ok,
        ms: Some(p50),
        note: Some(format!(
            "subtract of two offset spheres, {tris} triangles each, CPU cook, {} runs: p50 {p50:.1} ms, max {max:.1} ms, result {} triangles; target ≤ 150 ms on the RTX 4070 PC's CPU",
            ms.len(),
            out.prims()
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
