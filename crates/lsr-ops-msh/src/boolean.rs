//! Boolean: union, intersection and difference of two closed meshes with
//! the Manifold algorithm (`manifold-rust`, Apache-2.0, PRD A74): exact
//! symbolic-perturbation predicates, so the result is a closed manifold
//! welded by index, with no T-junctions. Inputs that are closed but not
//! manifold (edges shared by more than two faces, doubled sheets) go
//! through the library's robust engine; inputs that are not closed at all
//! are a node error. Manifold inputs that intersect themselves take the
//! exact engine as they are (no self-intersection scan, see `run`).
//!
//! Each input is triangulated and handed over as a `MeshGL` whose extra
//! vertex properties are the float point attributes, so the library
//! interpolates them onto new vertices. Each input is one run with its own
//! original ID, which says which input every output triangle came from.
//! The cook polls the job's cancel token, which is Manifold's own.

use lsr_core::geo::{AttrClass, names};
use lsr_core::registry::ParamSchema;
use manifold_rust::manifold::Manifold;
use manifold_rust::types::{BooleanEngine, Error, MeshGL, OpType};

use crate::cook::{CANCELLED, Job, Snap};
use crate::edit::triangulated;
use crate::mesh::{Column, Mesh};

pub fn params() -> Vec<ParamSchema> {
    vec![
        ParamSchema::menu(
            "operation",
            "Operation",
            &["union", "intersect", "subtract"],
            "union",
        )
        .help(
            "union: the space inside A or B; intersect: inside both; subtract: inside A but not B.",
        ),
    ]
}

/// Float point columns carried through the operation as Manifold vertex
/// properties, by name (`N` is dropped: it would be wrong on cut faces).
struct Layout {
    cols: Vec<(String, u8)>,
    width: usize,
}

impl Layout {
    fn new(meshes: &[&Mesh]) -> Self {
        let mut cols: Vec<(String, u8)> = Vec::new();
        for m in meshes {
            for c in &m.point_attrs {
                if c.floats().is_some()
                    && c.name != names::N
                    && cols.iter().all(|(n, _)| *n != c.name)
                {
                    cols.push((c.name.clone(), c.comps));
                }
            }
        }
        let width = cols.iter().map(|(_, c)| usize::from(*c)).sum();
        Self { cols, width }
    }

    /// Per column, `m`'s values (or `None`: the mesh lacks it, so a default).
    fn sources<'a>(&self, m: &'a Mesh) -> Vec<Option<&'a [f32]>> {
        self.cols
            .iter()
            .map(|(name, comps)| {
                m.point_attr(name)
                    .filter(|c| c.comps == *comps)
                    .and_then(Column::floats)
            })
            .collect()
    }

    /// Appends point `i`'s properties (after its position) to `out`.
    fn push(&self, sources: &[Option<&[f32]>], i: usize, out: &mut Vec<f32>) {
        for ((name, comps), src) in self.cols.iter().zip(sources) {
            let k = usize::from(*comps);
            match src {
                Some(v) => out.extend_from_slice(&v[i * k..(i + 1) * k]),
                None => {
                    // A missing colour or scale is one, everything else zero.
                    let one = name == names::CD || name == names::PSCALE;
                    out.extend(std::iter::repeat_n(if one { 1.0 } else { 0.0 }, k));
                }
            }
        }
    }
}

/// `m`'s closed polygons, triangulated, as a `MeshGL` of one run `id`, with
/// coincident vertices merged (points split for attributes still weld).
fn mesh_gl(m: &Mesh, layout: &Layout, id: u32, job: &Job) -> Result<MeshGL, String> {
    let t = triangulated(m, job)?;
    let stride = 3 + layout.width;
    let sources = layout.sources(&t);
    let mut gl = MeshGL {
        num_prop: stride as u32,
        run_original_id: vec![id],
        ..MeshGL::default()
    };
    let mut slot = vec![u32::MAX; t.points()];
    for f in 0..t.prims() {
        job.every(f)?;
        if !t.closed[f] || t.prim(f).len() != 3 {
            continue;
        }
        for &i in t.prim(f) {
            let i = i as usize;
            if slot[i] == u32::MAX {
                slot[i] = (gl.vert_properties.len() / stride) as u32;
                gl.vert_properties.extend_from_slice(&t.p[i]);
                layout.push(&sources, i, &mut gl.vert_properties);
            }
            gl.tri_verts.push(slot[i]);
        }
    }
    gl.merge();
    Ok(gl)
}

/// Imports one input: the solid, and whether it needs the robust engine
/// (it is closed but not manifold). Empty when it has no closed polygons.
fn solid(gl: &MeshGL, label: &str) -> Result<(Manifold, bool), String> {
    if gl.tri_verts.is_empty() {
        return Ok((Manifold::empty(), false));
    }
    let s = Manifold::from_mesh_gl(gl);
    if matches!(s.status(), Error::NoError) {
        return Ok((s, false));
    }
    // Closed non-manifold input is kept as soup for the robust engine.
    let s = Manifold::from_mesh_gl_robust(gl);
    match s.status() {
        Error::NoError => Ok((s, true)),
        Error::NotClosed => Err(format!(
            "input {label} is not a closed surface (it has holes or open edges), so it has no inside; close it first (e.g. Fuse, or Clean)"
        )),
        e => Err(format!("input {label} cannot be read as a solid ({e:?})")),
    }
}

/// The boolean's result as a mesh: points welded by position, triangles
/// with the groups `from_a` / `from_b` from each run's original ID.
fn from_gl(gl: &MeshGL, layout: &Layout, id_a: u32, job: &Job) -> Result<Mesh, String> {
    let stride = gl.num_prop as usize;
    let verts = gl.vert_properties.len() / stride.max(1);
    // The export splits a position into several vertices where properties
    // differ and lists them in the merge vectors; weld them back.
    let mut to: Vec<u32> = (0..verts as u32).collect();
    for (&f, &t) in gl.merge_from_vert.iter().zip(&gl.merge_to_vert) {
        to[f as usize] = t;
    }
    let mut point = vec![u32::MAX; verts];
    let mut out = Mesh::new();
    let mut cols: Vec<Vec<f32>> = layout
        .cols
        .iter()
        .map(|(_, c)| Vec::with_capacity(verts * usize::from(*c)))
        .collect();
    let mut id_of = |v: usize, out: &mut Mesh, cols: &mut Vec<Vec<f32>>| -> u32 {
        let mut v = v;
        while to[v] as usize != v {
            v = to[v] as usize;
        }
        if point[v] == u32::MAX {
            let row = &gl.vert_properties[v * stride..(v + 1) * stride];
            point[v] = out.add_point([row[0], row[1], row[2]]);
            let mut at = 3;
            for ((_, comps), col) in layout.cols.iter().zip(cols.iter_mut()) {
                let k = usize::from(*comps);
                col.extend_from_slice(&row[at..at + k]);
                at += k;
            }
        }
        point[v]
    };
    let mut from_a = Vec::with_capacity(gl.tri_verts.len() / 3);
    for (r, &orig) in gl.run_original_id.iter().enumerate() {
        let (lo, hi) = (
            gl.run_index[r] as usize / 3,
            gl.run_index[r + 1] as usize / 3,
        );
        for tri in lo..hi {
            job.every(tri)?;
            let ids =
                [0, 1, 2].map(|k| id_of(gl.tri_verts[3 * tri + k] as usize, &mut out, &mut cols));
            if ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2] {
                continue;
            }
            out.add_polygon(&ids);
            from_a.push(orig == id_a);
        }
    }
    out.point_attrs = layout
        .cols
        .iter()
        .zip(cols)
        .map(|((name, comps), v)| Column::f32(name, *comps, v))
        .collect();
    out.set_group(AttrClass::Prim, "from_a", &from_a)?;
    let from_b: Vec<bool> = from_a.iter().map(|a| !a).collect();
    out.set_group(AttrClass::Prim, "from_b", &from_b)?;
    crate::mesh::limits::check(
        out.points() as u64,
        out.prims() as u64,
        out.corners.len() as u64,
    )?;
    Ok(out)
}

pub fn run(s: &Snap, inputs: &[Option<Mesh>], job: &Job) -> Result<Mesh, String> {
    let get = |i: usize| inputs.get(i).and_then(Option::as_ref);
    let op = match s.str("operation") {
        "intersect" => OpType::Intersect,
        "subtract" => OpType::Subtract,
        _ => OpType::Add,
    };
    let (a, b) = match (get(0), get(1)) {
        (Some(a), Some(b)) => (a, b),
        (Some(a), None) => return Ok(a.clone()),
        (None, Some(b)) if op == OpType::Add => return Ok(b.clone()),
        _ => return Ok(Mesh::new()),
    };
    let layout = Layout::new(&[a, b]);
    // Two consecutive IDs, so A's triangles sort before B's in the export
    // whatever other cooks reserve meanwhile.
    let id_a = Manifold::reserve_ids(2);
    let (ma, soup_a) = solid(&mesh_gl(a, &layout, id_a, job)?, "A")?;
    let (mb, soup_b) = solid(&mesh_gl(b, &layout, id_a + 1, job)?, "B")?;
    job.check()?;
    // Manifold inputs take the exact engine directly. `Auto` would first scan
    // both for self-intersections, which costs more than the Boolean itself
    // at 100k triangles a side (PT-12); a self-intersecting manifold input is
    // taken as is, as before.
    let engine = if soup_a || soup_b {
        BooleanEngine::Robust
    } else {
        BooleanEngine::Exact
    };
    let r = ma.boolean_with_engine_and_token(&mb, op, engine, Some(job.token()));
    match r.status() {
        Error::NoError => {}
        Error::Cancelled => return Err(CANCELLED.into()),
        e => return Err(format!("the Boolean failed ({e:?})")),
    }
    from_gl(&r.get_mesh_gl(-1), &layout, id_a, job)
}
