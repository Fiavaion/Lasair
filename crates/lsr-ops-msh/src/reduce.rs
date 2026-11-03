//! PolyReduce: quadric error metric edge collapse (Garland & Heckbert,
//! "Surface Simplification Using Quadric Error Metrics", SIGGRAPH 1997).

use lsr_core::geo::names;
use lsr_core::registry::ParamSchema;

use crate::cook::{Job, Snap};
use crate::edit::triangulated;
use crate::mesh::{Column, Interp, Mesh, v3};
use crate::normals::point_normals;

pub fn params() -> Vec<ParamSchema> {
    vec![
        ParamSchema::float("percent", "Keep (%)", 25.0, Some(0.0), Some(100.0))
            .help("Share of the triangles to keep."),
        ParamSchema::float("tolerance", "Tolerance", 0.0, Some(0.0), None).help(
            "Stops earlier once the cheapest collapse would move the surface by more than about this distance (0: no limit).",
        ),
        ParamSchema::toggle("keep_boundary", "Keep Boundary", true)
            .help("Open edges (holes and borders) do not move."),
    ]
}

/// A symmetric 4×4 quadric: xx xy xz xw yy yz yw zz zw ww.
type Quadric = [f64; 10];

fn plane_quadric(n: [f64; 3], d: f64, w: f64) -> Quadric {
    let [a, b, c] = n;
    [
        a * a,
        a * b,
        a * c,
        a * d,
        b * b,
        b * c,
        b * d,
        c * c,
        c * d,
        d * d,
    ]
    .map(|x| x * w)
}

fn add_q(a: &Quadric, b: &Quadric) -> Quadric {
    std::array::from_fn(|i| a[i] + b[i])
}

fn error(q: &Quadric, v: [f64; 3]) -> f64 {
    let [x, y, z] = v;
    q[0] * x * x
        + 2.0 * q[1] * x * y
        + 2.0 * q[2] * x * z
        + 2.0 * q[3] * x
        + q[4] * y * y
        + 2.0 * q[5] * y * z
        + 2.0 * q[6] * y
        + q[7] * z * z
        + 2.0 * q[8] * z
        + q[9]
}

/// The position minimising `q`, if the 3×3 system is well conditioned.
fn optimum(q: &Quadric) -> Option<[f64; 3]> {
    let m = [[q[0], q[1], q[2]], [q[1], q[4], q[5]], [q[2], q[5], q[7]]];
    let r = [-q[3], -q[6], -q[8]];
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let scale = m.iter().flatten().fold(0.0f64, |a, &b| a.max(b.abs()));
    if det.abs() <= 1e-10 * scale.powi(3).max(1e-300) {
        return None;
    }
    let col = |k: usize| {
        let mut a = m;
        for i in 0..3 {
            a[i][k] = r[i];
        }
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    Some([col(0) / det, col(1) / det, col(2) / det])
}

/// A cost as a `u64` that sorts like `f64::total_cmp`.
fn cost_key(c: f64) -> u64 {
    let b = c.to_bits();
    if b >> 63 == 1 { !b } else { b | 1 << 63 }
}

fn key_cost(k: u64) -> f64 {
    f64::from_bits(if k >> 63 == 1 { k & !(1 << 63) } else { !k })
}

const NONE: u32 = u32::MAX;

/// A queued edge: (cost key, a << 32 | b, edge).
type Item = (u64, u64, u32);

fn edge_key(a: u32, b: u32) -> u64 {
    (u64::from(a) << 32) | u64::from(b)
}

/// The collapse queue: a 4-ary min-heap of edges on (cost, a, b), with each
/// edge's slot so its key changes in place. Every edge is in it at most once
/// and the order is total, so the pops are deterministic.
struct Queue {
    items: Vec<Item>,
    slot: Vec<u32>,
}

impl Queue {
    fn new(edges: usize) -> Self {
        Self {
            items: Vec::with_capacity(edges),
            slot: vec![NONE; edges],
        }
    }

    fn less(x: Item, y: Item) -> bool {
        (x.0, x.1) < (y.0, y.1)
    }

    fn place(&mut self, i: usize, it: Item) {
        self.items[i] = it;
        self.slot[it.2 as usize] = i as u32;
    }

    fn up(&mut self, mut i: usize) {
        let it = self.items[i];
        while i > 0 {
            let p = (i - 1) / 4;
            if !Self::less(it, self.items[p]) {
                break;
            }
            let q = self.items[p];
            self.place(i, q);
            i = p;
        }
        self.place(i, it);
    }

    fn down(&mut self, mut i: usize) {
        let it = self.items[i];
        let n = self.items.len();
        loop {
            let first = 4 * i + 1;
            if first >= n {
                break;
            }
            let mut c = first;
            for k in first + 1..(first + 4).min(n) {
                if Self::less(self.items[k], self.items[c]) {
                    c = k;
                }
            }
            if !Self::less(self.items[c], it) {
                break;
            }
            let q = self.items[c];
            self.place(i, q);
            i = c;
        }
        self.place(i, it);
    }

    /// Queues an edge; call once per edge, then `heapify`.
    fn push_unordered(&mut self, e: u32, cost: u64, ab: u64) {
        self.items.push((cost, ab, e));
    }

    /// Builds the heap from the unordered items in linear time.
    fn heapify(&mut self) {
        for (i, it) in self.items.iter().enumerate() {
            self.slot[it.2 as usize] = i as u32;
        }
        for i in (0..self.items.len().div_ceil(4)).rev() {
            self.down(i);
        }
    }

    fn set(&mut self, e: u32, cost: u64, ab: u64) {
        let it = (cost, ab, e);
        let s = self.slot[e as usize];
        if s == NONE {
            self.items.push(it);
            self.up(self.items.len() - 1);
        } else {
            let i = s as usize;
            let old = self.items[i];
            self.items[i] = it;
            if Self::less(it, old) {
                self.up(i);
            } else {
                self.down(i);
            }
        }
    }

    fn remove(&mut self, e: u32) {
        let s = self.slot[e as usize];
        if s == NONE {
            return;
        }
        self.slot[e as usize] = NONE;
        let i = s as usize;
        let last = self.items.pop().expect("a queued edge");
        if i < self.items.len() {
            self.place(i, last);
            self.up(i);
            let j = self.slot[last.2 as usize] as usize;
            self.down(j);
        }
    }

    fn pop(&mut self) -> Option<Item> {
        let top = *self.items.first()?;
        self.remove(top.2);
        Some(top)
    }
}

/// One short list per point, all in one buffer. A list that outgrows its
/// room moves to the end of the buffer with twice the room.
struct Lists<T> {
    data: Vec<T>,
    /// (start, length, room) per point.
    span: Vec<[u32; 3]>,
}

impl<T: Copy + Default> Lists<T> {
    /// Lists with room for `count[v]` items plus a little slack.
    fn with_room(count: &[u32]) -> Self {
        let mut span = Vec::with_capacity(count.len());
        let mut start = 0u32;
        for &c in count {
            span.push([start, 0, c + 2]);
            start += c + 2;
        }
        Self {
            data: vec![T::default(); start as usize],
            span,
        }
    }

    fn push(&mut self, v: u32, x: T) {
        let [start, len, room] = self.span[v as usize];
        let start = if len == room {
            let to = self.data.len();
            let (s, l) = (start as usize, len as usize);
            self.data.extend_from_within(s..s + l);
            self.data.resize(to + 2 * l.max(2), T::default());
            self.span[v as usize] = [to as u32, len, 2 * len.max(2)];
            to as u32
        } else {
            start
        };
        self.data[(start + len) as usize] = x;
        self.span[v as usize][1] = len + 1;
    }

    fn swap_remove(&mut self, v: u32, k: usize) {
        let list = &mut self[v];
        let last = list.len() - 1;
        list.swap(k, last);
        self.span[v as usize][1] -= 1;
    }

    fn truncate(&mut self, v: u32, len: usize) {
        self.span[v as usize][1] = len as u32;
    }

    /// Moves `v`'s list into `out`, leaving it empty.
    fn take(&mut self, v: u32, out: &mut Vec<T>) {
        out.clear();
        out.extend_from_slice(&self[v]);
        self.truncate(v, 0);
    }
}

impl<T> std::ops::Index<u32> for Lists<T> {
    type Output = [T];
    fn index(&self, v: u32) -> &[T] {
        let [start, len, _] = self.span[v as usize];
        &self.data[start as usize..(start + len) as usize]
    }
}

impl<T> std::ops::IndexMut<u32> for Lists<T> {
    fn index_mut(&mut self, v: u32) -> &mut [T] {
        let [start, len, _] = self.span[v as usize];
        &mut self.data[start as usize..(start + len) as usize]
    }
}

fn tri_normal(pos: &[[f64; 3]], t: [u32; 3]) -> [f64; 3] {
    let [a, b, c] = t.map(|i| pos[i as usize]);
    v3::cross(v3::sub(b, a), v3::sub(c, a))
}

/// Each point's triangles.
fn triangles_at(tris: &[[u32; 3]], n: usize) -> Lists<u32> {
    let mut count = vec![0u32; n];
    for tri in tris {
        for &v in tri {
            count[v as usize] += 1;
        }
    }
    let mut around = Lists::with_room(&count);
    for (t, tri) in tris.iter().enumerate() {
        for &v in tri {
            around.push(v, t as u32);
        }
    }
    around
}

/// An edge (a, b), a < b, and the one triangle using it.
type BoundaryEdge = ((u32, u32), u32);

/// The edges of `tris` in (a, b) order, a < b, and the boundary edges (used
/// by one triangle).
fn edges(tris: &[[u32; 3]], around: &Lists<u32>) -> (Vec<(u32, u32)>, Vec<BoundaryEdge>) {
    let mut ends = Vec::new();
    let mut boundary = Vec::new();
    let mut uses: Vec<(u32, u32)> = Vec::new();
    for a in 0..around.span.len() as u32 {
        uses.clear();
        let mut last = NONE;
        for &t in &around[a] {
            if t == last {
                continue;
            }
            last = t;
            let tri = tris[t as usize];
            for i in 0..3 {
                let (u, v) = (tri[i], tri[(i + 1) % 3]);
                if u != v && u.min(v) == a {
                    uses.push((u.max(v), t));
                }
            }
        }
        uses.sort_unstable();
        let mut i = 0;
        while i < uses.len() {
            let (b, t) = uses[i];
            let mut j = i + 1;
            while j < uses.len() && uses[j].0 == b {
                j += 1;
            }
            ends.push((a, b));
            if j - i == 1 {
                boundary.push(((a, b), t));
            }
            i = j;
        }
    }
    (ends, boundary)
}

/// A set of triangles being reduced, with its own point numbering.
struct Work {
    pos: Vec<[f64; 3]>,
    quad: Vec<Quadric>,
    locked: Vec<bool>,
    /// Points shared with other chunks: they neither move nor go.
    keep: Vec<bool>,
    tris: Vec<[u32; 3]>,
    alive: Vec<bool>,
}

impl Work {
    /// The cost of collapsing edge (a, b) and where the point goes.
    fn plan(&self, a: u32, b: u32) -> Option<(f64, [f64; 3])> {
        let (a, b) = (a as usize, b as usize);
        let la = self.locked[a] || self.keep[a];
        let lb = self.locked[b] || self.keep[b];
        let q = add_q(&self.quad[a], &self.quad[b]);
        let (pa, pb) = (self.pos[a], self.pos[b]);
        let target = match (la, lb) {
            (true, true) => return None,
            (true, false) => pa,
            (false, true) => pb,
            _ => optimum(&q).unwrap_or_else(|| {
                let mid = v3::scale(v3::add(pa, pb), 0.5);
                *[pa, pb, mid]
                    .iter()
                    .min_by(|x, y| error(&q, **x).total_cmp(&error(&q, **y)))
                    .expect("three candidates")
            }),
        };
        Some((error(&q, target).max(0.0), target))
    }
}

/// Collapses edges, cheapest first, until `target` triangles are left or
/// the cheapest would cost more than `tol`² (when `tol` > 0).
fn collapse(w: &mut Work, target: usize, tol: f64, job: &Job) -> Result<(), String> {
    let n = w.pos.len();
    let mut live = w.alive.iter().filter(|&&a| a).count();
    let mut around = triangles_at(&w.tris, n);
    let (ends, _) = edges(&w.tris, &around);
    // Each point's edges: (other end, edge).
    let mut count = vec![0u32; n];
    for &(a, b) in &ends {
        count[a as usize] += 1;
        count[b as usize] += 1;
    }
    let mut adj: Lists<(u32, u32)> = Lists::with_room(&count);
    for (e, &(a, b)) in ends.iter().enumerate() {
        adj.push(a, (b, e as u32));
        adj.push(b, (a, e as u32));
    }
    drop(count);
    let mut queue = Queue::new(ends.len());
    for (e, &(a, b)) in ends.iter().enumerate() {
        if let Some((cost, _)) = w.plan(a, b) {
            queue.push_unordered(e as u32, cost_key(cost), edge_key(a, b));
        }
    }
    drop(ends);
    queue.heapify();
    let mut steps = 0usize;
    // Per-point marks for neighbour sets; a new stamp clears them all.
    let mut mark = vec![0u32; n];
    let mut stamp = 0u32;
    let mut moved_tris: Vec<u32> = Vec::new();
    let mut moved_edges: Vec<(u32, u32)> = Vec::new();
    while live > target {
        let Some((key, ab, _)) = queue.pop() else {
            break;
        };
        steps += 1;
        job.every(steps)?;
        let (a, b) = ((ab >> 32) as u32, ab as u32);
        if tol > 0.0 && key_cost(key) > tol * tol {
            break;
        }
        let Some((_, target_pos)) = w.plan(a, b) else {
            continue;
        };
        // b goes into a, unless only b must stay.
        let (a, b) = if w.keep[b as usize] && !w.keep[a as usize] {
            (b, a)
        } else {
            (a, b)
        };
        // Link condition: the shared neighbours are exactly the opposite
        // corners of the triangles on the edge. (A point's edges are
        // exactly its live-triangle neighbours.)
        stamp += 1;
        for &(x, _) in &adj[a] {
            mark[x as usize] = stamp;
        }
        let shared = adj[b]
            .iter()
            .filter(|&&(x, _)| mark[x as usize] == stamp)
            .count();
        let on_edge = around[a]
            .iter()
            .filter(|&&t| w.alive[t as usize] && w.tris[t as usize].contains(&b))
            .count();
        if shared != on_edge || on_edge == 0 {
            continue;
        }
        // No triangle may flip or collapse.
        let mut ok = true;
        for &v in &[a, b] {
            for &t in &around[v] {
                let t = t as usize;
                let tri = w.tris[t];
                if !w.alive[t] || (tri.contains(&a) && tri.contains(&b)) {
                    continue;
                }
                let before = tri_normal(&w.pos, tri);
                let [p0, p1, p2] = tri.map(|x| {
                    if x == a || x == b {
                        target_pos
                    } else {
                        w.pos[x as usize]
                    }
                });
                let after = v3::cross(v3::sub(p1, p0), v3::sub(p2, p0));
                let (lb, la) = (v3::len(before), v3::len(after));
                if la <= 1e-12 * lb.max(1e-30) || v3::dot(before, after) < 0.2 * lb * la {
                    ok = false;
                    break;
                }
            }
            if !ok {
                break;
            }
        }
        if !ok {
            continue;
        }
        // Collapse b into a.
        w.pos[a as usize] = target_pos;
        w.quad[a as usize] = add_q(&w.quad[a as usize], &w.quad[b as usize]);
        w.locked[a as usize] |= w.locked[b as usize];
        // Triangles on the edge die (and leave their third corner's list);
        // b's others move to a.
        let mut k = 0;
        while k < around[a].len() {
            let t = around[a][k];
            let tri = w.tris[t as usize];
            if w.alive[t as usize] && tri.contains(&b) {
                w.alive[t as usize] = false;
                live -= 1;
                for &x in &tri {
                    if x != a && x != b {
                        let i = around[x].iter().position(|&u| u == t).expect("t at x");
                        around.swap_remove(x, i);
                    }
                }
            }
            if w.alive[t as usize] {
                k += 1;
            } else {
                around.swap_remove(a, k);
            }
        }
        around.take(b, &mut moved_tris);
        for &t in &moved_tris {
            let ti = t as usize;
            if !w.alive[ti] {
                continue;
            }
            if w.tris[ti].contains(&a) {
                w.alive[ti] = false;
                live -= 1;
            } else {
                for x in &mut w.tris[ti] {
                    if *x == b {
                        *x = a;
                    }
                }
                around.push(a, t);
            }
        }
        // Move b's edges to a; an edge a already has is merged away.
        adj.take(b, &mut moved_edges);
        for &(x, f) in &moved_edges {
            if x == a {
                let k = adj[a].iter().position(|&(_, g)| g == f).expect("edge at a");
                adj.swap_remove(a, k);
                continue;
            }
            let k = adj[x].iter().position(|&(_, g)| g == f).expect("edge at x");
            if adj[a].iter().any(|&(y, _)| y == x) {
                adj.swap_remove(x, k);
                queue.remove(f);
            } else {
                adj[x][k].0 = a;
                adj.push(a, (x, f));
            }
        }
        // Every edge at a changes cost; an edge no live triangle uses any
        // more is dropped.
        stamp += 1;
        for &t in &around[a] {
            for &x in &w.tris[t as usize] {
                mark[x as usize] = stamp;
            }
        }
        let mut k = 0;
        while k < adj[a].len() {
            let (x, f) = adj[a][k];
            if mark[x as usize] != stamp {
                adj.swap_remove(a, k);
                let i = adj[x].iter().position(|&(_, g)| g == f).expect("edge at x");
                adj.swap_remove(x, i);
                queue.remove(f);
                continue;
            }
            let (lo, hi) = (a.min(x), a.max(x));
            match w.plan(lo, hi) {
                Some((cost, _)) => queue.set(f, cost_key(cost), edge_key(lo, hi)),
                None => queue.remove(f),
            }
            k += 1;
        }
    }
    Ok(())
}

/// Meshes from this many triangles are reduced in chunks of about
/// [`CHUNK`] triangles first, each small enough to stay in cache, on
/// several threads; one pass over the whole mesh then finishes.
const CHUNKED: usize = 2 * CHUNK;
const CHUNK: usize = 1 << 16;
/// How far above the share the chunks stop, leaving the last collapses to
/// the pass over the whole mesh (where the seams between chunks can go).
const CHUNK_SLACK: f64 = 2.0;

/// The triangles in Z-order of their centres on a 32³ grid over the
/// points' bounds (a counting sort, so equal cells keep index order).
fn spatial_order(pos: &[[f64; 3]], tris: &[[u32; 3]]) -> Vec<u32> {
    const BITS: u32 = 5;
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for p in pos {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let side = (1u32 << BITS) as f64;
    let cell = |c: [f64; 3]| -> usize {
        let mut key = 0usize;
        let q: [u32; 3] = std::array::from_fn(|i| {
            let span = hi[i] - lo[i];
            let t = if span > 0.0 {
                (c[i] - lo[i]) / span
            } else {
                0.0
            };
            ((t * side) as u32).min((1 << BITS) - 1)
        });
        for bit in 0..BITS {
            for (i, &qi) in q.iter().enumerate() {
                key |= (((qi >> bit) & 1) as usize) << (3 * bit as usize + i);
            }
        }
        key
    };
    let keys: Vec<usize> = tris
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| pos[i as usize]);
            cell(v3::scale(v3::add(v3::add(a, b), c), 1.0 / 3.0))
        })
        .collect();
    let mut start = vec![0usize; (1 << (3 * BITS)) + 1];
    for &k in &keys {
        start[k + 1] += 1;
    }
    for i in 1..start.len() {
        start[i] += start[i - 1];
    }
    let mut order = vec![0u32; tris.len()];
    for (t, &k) in keys.iter().enumerate() {
        order[start[k]] = t as u32;
        start[k] += 1;
    }
    order
}

/// What one chunk leaves: its triangles (source triangle, points), its
/// own points (point, position, quadric, locked), and for the points it
/// shares, what their quadrics gained (point, gain, locked).
struct ChunkOut {
    tris: Vec<(u32, [u32; 3])>,
    own: Vec<(u32, [f64; 3], Quadric, bool)>,
    shared: Vec<(u32, Quadric, bool)>,
}

/// The whole mesh, as every chunk reads it.
struct Whole<'a> {
    pos: &'a [[f64; 3]],
    quad: &'a [Quadric],
    locked: &'a [bool],
    seam: &'a [bool],
    tris: &'a [[u32; 3]],
}

fn reduce_chunk(
    g: &Whole,
    chunk: &[u32],
    share: f64,
    tol: f64,
    job: &Job,
) -> Result<ChunkOut, String> {
    let mut points: Vec<u32> = chunk.iter().flat_map(|&t| g.tris[t as usize]).collect();
    points.sort_unstable();
    points.dedup();
    let local = |v: u32| points.binary_search(&v).expect("a chunk point") as u32;
    let mut w = Work {
        pos: points.iter().map(|&v| g.pos[v as usize]).collect(),
        quad: points.iter().map(|&v| g.quad[v as usize]).collect(),
        locked: points.iter().map(|&v| g.locked[v as usize]).collect(),
        keep: points.iter().map(|&v| g.seam[v as usize]).collect(),
        tris: chunk
            .iter()
            .map(|&t| g.tris[t as usize].map(local))
            .collect(),
        alive: vec![true; chunk.len()],
    };
    let target = ((chunk.len() as f64) * share).round() as usize;
    collapse(&mut w, target, tol, job)?;
    let mut out = ChunkOut {
        tris: Vec::new(),
        own: Vec::new(),
        shared: Vec::new(),
    };
    for (k, &t) in chunk.iter().enumerate() {
        if w.alive[k] {
            out.tris.push((t, w.tris[k].map(|v| points[v as usize])));
        }
    }
    for (i, &v) in points.iter().enumerate() {
        if w.keep[i] {
            let q = &g.quad[v as usize];
            let gain = std::array::from_fn(|j| w.quad[i][j] - q[j]);
            out.shared.push((v, gain, w.locked[i]));
        } else {
            out.own.push((v, w.pos[i], w.quad[i], w.locked[i]));
        }
    }
    Ok(out)
}

/// Reduces `tris` in spatial chunks on several threads, then the whole
/// mesh; the result does not depend on the number of threads. Returns the
/// surviving (source triangle, points), in source order; `pos` holds the
/// points' new positions.
fn reduce_chunked(
    pos: &mut [[f64; 3]],
    mut quad: Vec<Quadric>,
    mut locked: Vec<bool>,
    tris: &[[u32; 3]],
    target: usize,
    tol: f64,
    job: &Job,
) -> Result<Vec<(u32, [u32; 3])>, String> {
    let n = pos.len();
    let order = spatial_order(pos, tris);
    let chunks = tris.len().div_ceil(CHUNK);
    let range = |c: usize| &order[c * tris.len() / chunks..(c + 1) * tris.len() / chunks];
    // A point on triangles of two chunks is a seam point.
    let mut owner = vec![NONE; n];
    let mut seam = vec![false; n];
    for c in 0..chunks {
        for &t in range(c) {
            for &v in &tris[t as usize] {
                let o = &mut owner[v as usize];
                if *o == NONE {
                    *o = c as u32;
                } else if *o != c as u32 {
                    seam[v as usize] = true;
                }
            }
        }
    }
    drop(owner);
    let share = (target as f64 / tris.len() as f64 * CHUNK_SLACK).min(1.0);
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 8)
        .min(chunks);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let g = Whole {
        pos,
        quad: &quad,
        locked: &locked,
        seam: &seam,
        tris,
    };
    let mut outs: Vec<Option<Result<ChunkOut, String>>> = (0..chunks).map(|_| None).collect();
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let c = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if c >= chunks {
                            break done;
                        }
                        done.push((c, reduce_chunk(&g, range(c), share, tol, job)));
                    }
                })
            })
            .collect();
        for wk in workers {
            for (c, out) in wk.join().expect("a PolyReduce chunk thread") {
                outs[c] = Some(out);
            }
        }
    });
    // Merge in chunk order.
    let mut left: Vec<(u32, [u32; 3])> = Vec::new();
    for out in outs {
        let out = out.expect("every chunk ran")?;
        for (v, p, q, l) in out.own {
            pos[v as usize] = p;
            quad[v as usize] = q;
            locked[v as usize] = l;
        }
        for (v, gain, l) in out.shared {
            quad[v as usize] = add_q(&quad[v as usize], &gain);
            locked[v as usize] |= l;
        }
        left.extend(out.tris);
    }
    left.sort_unstable_by_key(|&(t, _)| t);
    // The whole mesh, with the points still in use.
    let mut points: Vec<u32> = left.iter().flat_map(|&(_, tri)| tri).collect();
    points.sort_unstable();
    points.dedup();
    let local = |v: u32| points.binary_search(&v).expect("a point in use") as u32;
    let mut w = Work {
        pos: points.iter().map(|&v| pos[v as usize]).collect(),
        quad: points.iter().map(|&v| quad[v as usize]).collect(),
        locked: points.iter().map(|&v| locked[v as usize]).collect(),
        keep: vec![false; points.len()],
        tris: left.iter().map(|&(_, tri)| tri.map(local)).collect(),
        alive: vec![true; left.len()],
    };
    collapse(&mut w, target, tol, job)?;
    for (i, &v) in points.iter().enumerate() {
        pos[v as usize] = w.pos[i];
    }
    Ok(left
        .iter()
        .zip(&w.tris)
        .zip(&w.alive)
        .filter(|&(_, &alive)| alive)
        .map(|((&(t, _), tri), _)| (t, tri.map(|v| points[v as usize])))
        .collect())
}

pub fn run(s: &Snap, inputs: &[Option<Mesh>], job: &Job) -> Result<Mesh, String> {
    let Some(input) = inputs.first().and_then(Option::as_ref) else {
        return Ok(Mesh::new());
    };
    let m = triangulated(input, job)?;
    let had = m.point_attr(names::N).is_some();
    let n = m.points();
    let mut pos: Vec<[f64; 3]> = m.p.iter().map(|&p| v3::of(p)).collect();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    let mut src: Vec<u32> = Vec::new();
    let mut lines: Vec<usize> = Vec::new();
    for f in 0..m.prims() {
        let p = m.prim(f);
        if m.closed[f] && p.len() == 3 {
            tris.push([p[0], p[1], p[2]]);
            src.push(f as u32);
        } else if !m.closed[f] {
            lines.push(f);
        }
    }
    let target =
        ((tris.len() as f64) * s.f64("percent").clamp(0.0, 100.0) / 100.0).round() as usize;
    let tol = s.f64("tolerance").max(0.0);
    // Quadrics from the planes of each point's triangles (unweighted, so
    // the cost is a sum of squared distances and the tolerance a distance).
    let mut quad = vec![[0.0f64; 10]; n];
    for tri in &tris {
        let nn = v3::norm(tri_normal(&pos, *tri));
        let d = -v3::dot(nn, pos[tri[0] as usize]);
        let q = plane_quadric(nn, d, 1.0);
        for &v in tri {
            quad[v as usize] = add_q(&quad[v as usize], &q);
        }
    }
    let mut locked = vec![false; n];
    for &f in &lines {
        for &c in m.prim(f) {
            locked[c as usize] = true;
        }
    }
    let keep_boundary = s.bool("keep_boundary");
    let (_, boundary) = edges(&tris, &triangles_at(&tris, n));
    for ((a, b), t) in boundary {
        if keep_boundary {
            locked[a as usize] = true;
            locked[b as usize] = true;
        } else {
            // A plane through the edge, perpendicular to its triangle,
            // weighted heavily: the border keeps its shape.
            let nn = v3::norm(tri_normal(&pos, tris[t as usize]));
            let e = v3::sub(pos[b as usize], pos[a as usize]);
            let side = v3::norm(v3::cross(e, nn));
            let d = -v3::dot(side, pos[a as usize]);
            let q = plane_quadric(side, d, 100.0);
            quad[a as usize] = add_q(&quad[a as usize], &q);
            quad[b as usize] = add_q(&quad[b as usize], &q);
        }
    }
    let left: Vec<(u32, [u32; 3])> = if tris.len() >= CHUNKED {
        reduce_chunked(&mut pos, quad, locked, &tris, target, tol, job)?
    } else {
        let mut w = Work {
            pos,
            quad,
            locked,
            keep: vec![false; n],
            alive: vec![true; tris.len()],
            tris,
        };
        collapse(&mut w, target, tol, job)?;
        pos = w.pos;
        (0..w.tris.len() as u32)
            .filter(|&t| w.alive[t as usize])
            .map(|t| (t, w.tris[t as usize]))
            .collect()
    };
    // Rebuild: surviving triangles and polylines, compacted points.
    let mut out = Mesh::new();
    let mut map = vec![u32::MAX; n];
    let mut pts = Interp::new(&m.point_attrs, n);
    let mut id = |v: u32, out: &mut Mesh, pts: &mut Interp| -> u32 {
        if map[v as usize] == u32::MAX {
            map[v as usize] = out.add_point(v3::f32(pos[v as usize]));
            pts.copy(v);
        }
        map[v as usize]
    };
    let mut prims = Interp::new(&m.prim_attrs, left.len() + lines.len());
    for &(t, tri) in &left {
        let ids = tri.map(|v| id(v, &mut out, &mut pts));
        out.add_polygon(&ids);
        prims.copy(src[t as usize]);
    }
    for &f in &lines {
        let ids: Vec<u32> = m
            .prim(f)
            .iter()
            .map(|&v| id(v, &mut out, &mut pts))
            .collect();
        out.add_prim(&ids, false);
        prims.copy(f as u32);
    }
    out.point_attrs = pts.finish();
    out.prim_attrs = prims.finish();
    out.point_groups = m.point_groups.clone();
    out.prim_groups = m.prim_groups.clone();
    if had {
        let nn = point_normals(&out);
        out.set_point_attr(Column::vec3(names::N, &nn));
    }
    Ok(out)
}
