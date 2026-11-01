//! The network-generation eval's checker (`lasair agent eval`, M5 AC1): the
//! prompt file format and the scoring of the network the assistant left.
//! Pure functions of a network, so the reference solutions can be scored
//! without a model (see the tests).
//!
//! Every check is one point; a run is *valid* when it scores them all.
//!
//! - `min_nodes` (always one point), `max_nodes` (optional).
//! - `require`: one point per entry; some node's type starts with it.
//! - `paths`: one point per entry, `"img.noise > img.feedback > cmp.window"`;
//!   following wires (any number of other nodes between), a path visits
//!   nodes whose types start with each prefix, in that order. Channel
//!   bindings are not wires. A feedback loop may close a path (the same
//!   node can match two steps).
//! - `params`: one point per entry, `{type, name, eq | any_of | min | max |
//!   contains}`; some node whose type starts with `type` has the parameter
//!   `name` with a *stored* value meeting every constraint (the schema
//!   default when the node does not override it; for a parameter driven by
//!   an expression, the expression's last result is what is stored, so test
//!   those with `exprs`). `eq` compares numbers with a small tolerance,
//!   strings and toggles exactly, and vectors and colours component by
//!   component; `any_of` is a list of scalar alternatives for `eq`; `min`
//!   and `max` bound a number, or every component of a vector; `contains`
//!   is a case-insensitive substring of a string value.
//! - `exprs`: one point per entry, `{type, param?, contains?, bind?, from?}`;
//!   some node of that type has `param` (any parameter when omitted) driven:
//!   with `contains`, by an expression whose text contains it; with `bind =
//!   true`, by a channel binding; with neither, by either. `from` also
//!   requires the bound SIG node's type to start with it. `range = [lo,
//!   hi]`: the parameter is bound to a channel that swings between those
//!   values (within 5%): an LFO's offset ∓ amplitude, or a Math node's
//!   `to_min`..`to_max`. `period = s`: what drives it repeats every `s`
//!   seconds: a bound Timer's `length` or LFO's 1 / `frequency`, or an
//!   expression that names `s` (`time % 12`).
//! - `count`: one point per entry, `{type, min?, max?}`: the number of nodes
//!   of that type prefix lies in the range.
//! - `seed`: a project directory the run starts from (a copy), for debug and
//!   edit tasks; `keep`: one point per node name that must still exist.
//! - `animated`: something outside the channel nodes moves.
//! - Always one point: every node cooks without an error.
//!
//! A model error ends the run with one extra point missed.
//!
//! Every type prefix above (in `require`, a path step, `type`, `from`) may
//! list alternatives with `|`: `"pnt.noise|pnt.turbulence"` matches either,
//! for prompts a model can rightly answer more than one way.

use std::path::{Path, PathBuf};

use lsr_core::graph::{Network, Node, NodeId, Param};
use lsr_core::registry::Registry;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct Prompts {
    pub prompt: Vec<Prompt>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Prompt {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub require: Vec<String>,
    #[serde(default = "one")]
    pub min_nodes: usize,
    #[serde(default)]
    pub max_nodes: Option<usize>,
    /// Something must move: a parameter with an expression or a channel
    /// binding, or a SIG node wired into another node.
    #[serde(default)]
    pub animated: bool,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub params: Vec<ParamCheck>,
    #[serde(default)]
    pub exprs: Vec<ExprCheck>,
    #[serde(default)]
    pub count: Vec<CountCheck>,
    /// A project directory (relative to where `lasair` runs) the run starts
    /// from, instead of an empty project.
    #[serde(default)]
    pub seed: Option<PathBuf>,
    /// Node names that must still exist afterwards.
    #[serde(default)]
    pub keep: Vec<String>,
}

fn one() -> usize {
    1
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParamCheck {
    #[serde(rename = "type")]
    pub type_id: String,
    pub name: String,
    #[serde(default)]
    pub eq: Option<Value>,
    #[serde(default)]
    pub any_of: Vec<Value>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub contains: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExprCheck {
    #[serde(rename = "type")]
    pub type_id: String,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub bind: bool,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    #[serde(default)]
    pub period: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CountCheck {
    #[serde(rename = "type")]
    pub type_id: String,
    #[serde(default)]
    pub min: usize,
    #[serde(default)]
    pub max: Option<usize>,
}

/// The result for one run.
#[derive(Debug)]
pub(crate) struct Verdict {
    pub points: usize,
    pub max: usize,
    pub why: Vec<String>,
}

impl Verdict {
    pub fn valid(&self) -> bool {
        self.points == self.max
    }
}

impl Prompt {
    /// Scores `net` (cooked: `errors` are its node errors) against this
    /// prompt. `model_error` is the assistant's own failure, if any.
    pub fn judge(
        &self,
        net: &Network,
        registry: &Registry,
        errors: &[String],
        model_error: Option<&str>,
    ) -> Verdict {
        let mut v = Verdict {
            points: 0,
            max: 0,
            why: Vec::new(),
        };
        let mut check = |ok: bool, why: String| {
            v.max += 1;
            if ok {
                v.points += 1;
            } else {
                v.why.push(why);
            }
        };
        let starts = |n: &Node, p: &str| type_matches(&n.type_id, p);

        check(
            net.len() >= self.min_nodes,
            format!("{} nodes < {}", net.len(), self.min_nodes),
        );
        if let Some(m) = self.max_nodes {
            check(net.len() <= m, format!("{} nodes > {m}", net.len()));
        }
        for r in &self.require {
            check(net.nodes().any(|n| starts(n, r)), format!("missing {r}"));
        }
        for p in &self.paths {
            check(path_exists(net, p), format!("no path {p}"));
        }
        for c in &self.params {
            check(
                net.nodes()
                    .filter(|n| starts(n, &c.type_id))
                    .any(|n| param_ok(net, registry, n, c)),
                format!("no {}.{} {}", c.type_id, c.name, c.describe()),
            );
        }
        for c in &self.exprs {
            check(
                net.nodes()
                    .filter(|n| starts(n, &c.type_id))
                    .any(|n| expr_ok(net, registry, n, c)),
                format!("{} {} not driven as asked", c.type_id, c.describe()),
            );
        }
        for c in &self.count {
            let n = net.nodes().filter(|n| starts(n, &c.type_id)).count();
            check(
                n >= c.min && c.max.is_none_or(|m| n <= m),
                format!(
                    "{n} x {} (want {}..{})",
                    c.type_id,
                    c.min,
                    c.max.map_or("any".into(), |m| m.to_string())
                ),
            );
        }
        for k in &self.keep {
            check(net.node_by_name(k).is_some(), format!("lost node {k}"));
        }
        check(
            errors.is_empty(),
            format!(
                "{} node error(s): {}",
                errors.len(),
                errors.first().map_or("", String::as_str)
            ),
        );
        if self.animated {
            check(moves(net), "nothing is animated".into());
        }
        if let Some(e) = model_error {
            check(false, format!("model error: {e}"));
        }
        v
    }
}

impl ParamCheck {
    fn describe(&self) -> String {
        let mut s = Vec::new();
        if let Some(e) = &self.eq {
            s.push(format!("= {e}"));
        }
        if !self.any_of.is_empty() {
            s.push(format!(
                "in {}",
                serde_json::to_string(&self.any_of).unwrap_or_default()
            ));
        }
        if let Some(m) = self.min {
            s.push(format!(">= {m}"));
        }
        if let Some(m) = self.max {
            s.push(format!("<= {m}"));
        }
        if let Some(c) = &self.contains {
            s.push(format!("contains {c:?}"));
        }
        s.join(" ")
    }
}

impl ExprCheck {
    fn describe(&self) -> String {
        let mut s = self.param.clone().unwrap_or_else(|| "any param".into());
        if let Some(c) = &self.contains {
            s += &format!(" expression with {c:?}");
        }
        if self.bind {
            s += " bound";
        }
        if let Some(f) = &self.from {
            s += &format!(" from {f}");
        }
        if let Some([lo, hi]) = self.range {
            s += &format!(" over {lo}..{hi}");
        }
        if let Some(t) = self.period {
            s += &format!(" every {t} s");
        }
        s
    }
}

/// `t` starts with `pat`, or with one of its `|` alternatives.
fn type_matches(t: &str, pat: &str) -> bool {
    pat.split('|').any(|p| t.starts_with(p.trim()))
}

/// A path through the wires visiting nodes of each prefix in order.
fn path_exists(net: &Network, path: &str) -> bool {
    let steps: Vec<&str> = path.split('>').map(str::trim).collect();
    let matches = |id: NodeId, k: usize| {
        net.node(id)
            .is_some_and(|n| type_matches(&n.type_id, steps[k]))
    };
    // State (node, k): k steps matched, the last at `node` or upstream of it.
    let mut seen = std::collections::BTreeSet::new();
    let mut todo: Vec<(NodeId, usize)> = net
        .nodes()
        .filter(|n| matches(n.id, 0))
        .map(|n| (n.id, 1))
        .collect();
    while let Some((id, k)) = todo.pop() {
        if k == steps.len() {
            return true;
        }
        if !seen.insert((id, k)) {
            continue;
        }
        for w in net.wires().filter(|w| w.from.node == id) {
            let next = w.to.node;
            todo.push((next, k));
            if matches(next, k) {
                todo.push((next, k + 1));
            }
        }
    }
    false
}

fn num_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 + 1e-4 * b.abs()
}

fn value_eq(have: &Value, want: &Value) -> bool {
    match (have, want) {
        (Value::Number(a), Value::Number(b)) => {
            matches!((a.as_f64(), b.as_f64()), (Some(a), Some(b)) if num_eq(a, b))
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| value_eq(a, b))
        }
        (a, b) => a == b,
    }
}

/// Every numeric component of `v` (a number, or an array of numbers).
fn numbers(v: &Value) -> Option<Vec<f64>> {
    match v {
        Value::Number(n) => n.as_f64().map(|x| vec![x]),
        Value::Array(a) => a.iter().map(|x| x.as_f64()).collect(),
        _ => None,
    }
}

fn param_ok(net: &Network, registry: &Registry, n: &Node, c: &ParamCheck) -> bool {
    let Some(v) = net.param_value(registry, n.id, &c.name) else {
        return false;
    };
    if c.eq.as_ref().is_some_and(|e| !value_eq(v, e)) {
        return false;
    }
    if !c.any_of.is_empty() && !c.any_of.iter().any(|e| value_eq(v, e)) {
        return false;
    }
    if c.min.is_some() || c.max.is_some() {
        let Some(xs) = numbers(v) else {
            return false;
        };
        if xs
            .iter()
            .any(|x| c.min.is_some_and(|m| *x < m - 1e-9) || c.max.is_some_and(|m| *x > m + 1e-9))
        {
            return false;
        }
    }
    if let Some(s) = &c.contains {
        let Some(t) = v.as_str() else {
            return false;
        };
        if !t.to_lowercase().contains(&s.to_lowercase()) {
            return false;
        }
    }
    true
}

fn driven_ok(net: &Network, registry: &Registry, p: &Param, c: &ExprCheck) -> bool {
    let source = p.bind.as_ref().and_then(|b| net.node(b.node));
    let bound = || {
        source.is_some_and(|src| {
            c.from
                .as_ref()
                .is_none_or(|f| type_matches(&src.type_id, f))
        })
    };
    let ok = match (&c.contains, c.bind) {
        (Some(s), _) => p.expr.as_ref().is_some_and(|e| e.contains(s.as_str())),
        (None, true) => bound(),
        (None, false) => p.expr.is_some() || bound(),
    };
    let num = |n: &Node, name: &str| {
        net.param_value(registry, n.id, name)
            .and_then(Value::as_f64)
    };
    let close = |a: f64, b: f64| (a - b).abs() <= 0.05 * b.abs().max(1e-3);
    let in_range = c.range.is_none_or(|[lo, hi]| {
        source.is_some_and(|src| {
            let swing = if src.type_id.starts_with("sig.lfo") {
                num(src, "offset")
                    .zip(num(src, "amplitude"))
                    .map(|(o, a)| (o - a.abs(), o + a.abs()))
            } else if src.type_id.starts_with("sig.math") {
                num(src, "to_min").zip(num(src, "to_max"))
            } else {
                None
            };
            swing.is_some_and(|(a, b)| close(a.min(b), lo) && close(a.max(b), hi))
        })
    });
    // The channel and every SIG node upstream of it (a Math remap fed by
    // a Timer repeats with the Timer).
    let upstream = |src: &Node| {
        let mut seen = vec![src.id];
        let mut k = 0;
        while k < seen.len() {
            let id = seen[k];
            for w in net.wires().filter(|w| w.to.node == id) {
                if !seen.contains(&w.from.node)
                    && net
                        .node(w.from.node)
                        .is_some_and(|n| n.type_id.starts_with("sig."))
                {
                    seen.push(w.from.node);
                }
            }
            k += 1;
        }
        seen
    };
    let repeats = c.period.is_none_or(|t| match source {
        Some(src) => upstream(src)
            .into_iter()
            .filter_map(|id| net.node(id))
            .any(|n| {
                if n.type_id.starts_with("sig.timer") {
                    num(n, "length").is_some_and(|l| close(l, t))
                } else if n.type_id.starts_with("sig.lfo") {
                    num(n, "frequency").is_some_and(|f| f > 0.0 && close(1.0 / f, t))
                } else {
                    false
                }
            }),
        None => p.expr.as_ref().is_some_and(|e| {
            let t = format!("{t}");
            e.split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
                .any(|w| w == t || w.trim_end_matches(".0") == t)
        }),
    });
    ok && in_range && repeats
}

fn expr_ok(net: &Network, registry: &Registry, n: &Node, c: &ExprCheck) -> bool {
    match &c.param {
        Some(name) => n
            .params
            .get(name)
            .is_some_and(|p| driven_ok(net, registry, p, c)),
        None => n.params.values().any(|p| driven_ok(net, registry, p, c)),
    }
}

/// The motion must reach a node that is not itself a channel node (an LFO
/// with an expression that drives nothing does not count).
fn moves(net: &Network) -> bool {
    let sig = |id| {
        net.node(id)
            .is_some_and(|n: &Node| n.type_id.starts_with("sig."))
    };
    net.nodes().filter(|n| !sig(n.id)).any(|n| {
        n.params
            .values()
            .any(|p| p.expr.is_some() || p.bind.is_some())
    }) || net.wires().any(|w| sig(w.from.node) && !sig(w.to.node))
}

/// Copies a project directory (the files stay text).
pub(crate) fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let dest = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &dest)?;
        } else {
            std::fs::copy(e.path(), dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lsr_agent::Workspace;

    use super::*;
    use crate::serve::ServeWorkspace;

    const FILE: &str = "docs/milestones/m5/eval-prompts-hard.toml";

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn prompts() -> Vec<Prompt> {
        let text = std::fs::read_to_string(root().join(FILE)).unwrap();
        toml_edit::de::from_str::<Prompts>(&text).unwrap().prompt
    }

    /// Scores the project at `dir` as the eval would. Node errors come from
    /// a real cook, which needs a GPU; without one they are not judged.
    fn score(p: &Prompt, dir: &Path) -> (Verdict, bool) {
        let loaded = crate::project::load_checked(dir).unwrap();
        let net = loaded.project.main().cloned().unwrap();
        let tmp = std::env::temp_dir().join(format!("lsr-check-{}", std::process::id()));
        let copy = tmp.join(format!("{}.lasair", p.id));
        let _ = std::fs::remove_dir_all(&copy);
        copy_dir(dir, &copy).unwrap();
        let (errors, cooked) = match ServeWorkspace::open(&copy, false, false) {
            Ok(mut ws) => (
                ws.cook_errors()
                    .into_iter()
                    .map(|e| e.1)
                    .collect::<Vec<_>>(),
                true,
            ),
            Err(_) => (Vec::new(), false),
        };
        let _ = std::fs::remove_dir_all(&tmp);
        (p.judge(&net, &loaded.registry, &errors, None), cooked)
    }

    #[test]
    fn the_hard_prompts_parse_with_unique_ids() {
        let ps = prompts();
        assert!(ps.len() >= 15);
        let mut ids: Vec<&str> = ps.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ps.len());
    }

    #[test]
    fn the_original_twenty_prompts_still_parse() {
        let text =
            std::fs::read_to_string(root().join("docs/milestones/m5/eval-prompts.toml")).unwrap();
        let ps = toml_edit::de::from_str::<Prompts>(&text).unwrap().prompt;
        assert_eq!(ps.len(), 20);
        assert!(ps.iter().all(|p| p.seed.is_none() && p.paths.is_empty()));
    }

    #[test]
    fn every_reference_solution_scores_full_points() {
        let mut failures = Vec::new();
        for p in prompts() {
            let dir = root().join(format!("docs/milestones/m5/hard/solutions/{}.lasair", p.id));
            let (v, _) = score(&p, &dir);
            eprintln!("{}: {}/{}", p.id, v.points, v.max);
            if !v.valid() {
                failures.push(format!("{}: {}/{} {:?}", p.id, v.points, v.max, v.why));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// Networks models built that answer the prompt another right way (an
    /// LFO's own offset and amplitude instead of a Math remap, a time
    /// expression instead of a Timer, ...): `<id>--<model>.lasair`.
    #[test]
    fn right_answers_the_models_found_score_full_points() {
        let dir = root().join("docs/milestones/m5/hard/alternatives");
        let ps = prompts();
        let mut seen = 0;
        let mut failures = Vec::new();
        for e in std::fs::read_dir(&dir).unwrap() {
            let path = e.unwrap().path();
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let id = name.split("--").next().unwrap();
            let p = ps.iter().find(|p| p.id == id).expect(id);
            let (v, _) = score(p, &path);
            seen += 1;
            if !v.valid() {
                failures.push(format!("{name}: {}/{} {:?}", v.points, v.max, v.why));
            }
        }
        assert!(seen >= 4);
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn every_seed_as_it_stands_misses_points() {
        let mut seeded = 0;
        for p in prompts() {
            let Some(seed) = &p.seed else { continue };
            seeded += 1;
            assert!(
                seed.starts_with("docs/milestones/m5/hard/seeds"),
                "{}: seed in the wrong place",
                p.id
            );
            let (v, cooked) = score(&p, &root().join(seed));
            eprintln!("{} seed: {}/{}", p.id, v.points, v.max);
            if p.id == "h-fix-broken-chain" && cooked {
                // The seed's expression names a node that does not exist.
                assert!(
                    v.why.iter().any(|w| w.contains("node error")),
                    "{:?}",
                    v.why
                );
            }
            assert!(!v.valid(), "{}: the seed already passes", p.id);
            assert!(v.points > 0, "{}: the seed scores nothing", p.id);
        }
        assert!(seeded >= 5);
    }

    #[test]
    fn every_seed_keeps_the_names_it_is_asked_to_keep() {
        for p in prompts() {
            let Some(seed) = &p.seed else { continue };
            let loaded = crate::project::load_checked(&root().join(seed)).unwrap();
            let net = loaded.project.main().unwrap();
            for k in &p.keep {
                assert!(net.node_by_name(k).is_some(), "{}: seed lacks {k}", p.id);
            }
        }
    }

    #[test]
    fn every_solution_that_has_a_seed_keeps_its_named_nodes() {
        // Covered by full points (keep is a check), but only if keep is set.
        for p in prompts().iter().filter(|p| p.seed.is_some()) {
            assert!(!p.keep.is_empty(), "{}: seeded prompts must set keep", p.id);
        }
    }

    // The checker itself, on small hand-built networks.

    fn net(spec: &[(&str, &str)], wires: &[(usize, usize, &str)]) -> Network {
        use lsr_core::graph::{Edit, Endpoint, Wire};
        let registry = crate::project::library().0;
        let mut net = Network::default();
        let mut ids = Vec::new();
        for (name, ty) in spec {
            let id = net.alloc_id();
            net.apply(
                &registry,
                Edit::AddNode {
                    node: Node::new(id, *ty, *name),
                },
            )
            .unwrap();
            ids.push(id);
        }
        for (a, b, input) in wires {
            net.apply(
                &registry,
                Edit::Connect {
                    wire: Wire::new(
                        Endpoint::new(ids[*a], "out"),
                        Endpoint::new(ids[*b], *input),
                    ),
                },
            )
            .unwrap();
        }
        net
    }

    fn prompt(extra: &str) -> Prompt {
        toml_edit::de::from_str(&format!("id = \"t\"\ntext = \"t\"\n{extra}")).unwrap()
    }

    #[test]
    fn a_path_follows_wires_in_order_and_allows_nodes_between() {
        let n = net(
            &[
                ("a", "img.noise"),
                ("b", "img.blur"),
                ("c", "img.level"),
                ("d", "img.bloom"),
            ],
            &[(0, 1, "in"), (1, 2, "in"), (2, 3, "in")],
        );
        assert!(path_exists(&n, "img.noise > img.level > img.bloom"));
        assert!(path_exists(&n, "img.noise>img.bloom"));
        assert!(!path_exists(&n, "img.bloom > img.noise"));
        assert!(!path_exists(&n, "img.blur > img.noise"));
    }

    #[test]
    fn a_feedback_loop_can_close_a_path() {
        let n = net(
            &[
                ("f", "img.feedback"),
                ("t", "img.transform"),
                ("c", "img.composite"),
            ],
            &[(0, 1, "in"), (1, 2, "b"), (2, 0, "loop")],
        );
        assert!(path_exists(
            &n,
            "img.feedback > img.transform > img.composite > img.feedback"
        ));
    }

    #[test]
    fn points_are_one_per_check_and_the_old_fields_score_as_before() {
        let registry = crate::project::library().0;
        let n = net(&[("a", "img.noise"), ("b", "img.blur")], &[(0, 1, "in")]);
        let p = prompt("require = [\"img.noise\", \"img.blur\"]\nmin_nodes = 2");
        let v = p.judge(&n, &registry, &[], None);
        // min_nodes + two requires + no cook errors.
        assert_eq!((v.points, v.max), (4, 4));
        let p = prompt(
            "require = [\"img.noise\", \"img.kaleidoscope\"]\nmin_nodes = 3\nanimated = true",
        );
        let v = p.judge(&n, &registry, &["x: boom".into()], Some("rate limit"));
        assert_eq!((v.points, v.max), (1, 6));
        assert!(!v.valid());
    }

    #[test]
    fn param_constraints_use_defaults_numbers_strings_and_vectors() {
        let registry = crate::project::library().0;
        let n = net(&[("k", "img.kaleidoscope"), ("t", "img.tile")], &[]);
        let ok = |extra: &str| prompt(extra).judge(&n, &registry, &[], None).valid();
        // The schema default (6) counts.
        assert!(ok(
            "params = [{type = \"img.kaleidoscope\", name = \"segments\", eq = 6}]"
        ));
        assert!(!ok(
            "params = [{type = \"img.kaleidoscope\", name = \"segments\", eq = 8}]"
        ));
        assert!(ok(
            "params = [{type = \"img.kaleidoscope\", name = \"segments\", min = 4, max = 8}]"
        ));
        assert!(!ok(
            "params = [{type = \"img.kaleidoscope\", name = \"segments\", min = 7}]"
        ));
        assert!(ok(
            "params = [{type = \"img.tile\", name = \"repeat\", eq = [2, 2]}]"
        ));
        assert!(ok(
            "params = [{type = \"img.tile\", name = \"repeat\", min = 1, max = 2}]"
        ));
        assert!(!ok(
            "params = [{type = \"img.tile\", name = \"repeat\", eq = [2, 3]}]"
        ));
        assert!(ok(
            "params = [{type = \"img.tile\", name = \"mirror\", eq = false}]"
        ));
        assert!(ok(
            "params = [{type = \"img.kaleidoscope\", name = \"segments\", any_of = [4, 6]}]"
        ));
        assert!(!ok("count = [{type = \"img.tile\", min = 2}]"));
        assert!(ok(
            "count = [{type = \"img.tile\", min = 1, max = 1}]\nmax_nodes = 2\nkeep = [\"k\"]"
        ));
        assert!(!ok("max_nodes = 1"));
        assert!(!ok("keep = [\"gone\"]"));
    }

    #[test]
    fn expression_and_binding_checks() {
        let registry = crate::project::library().0;
        let mut n = net(&[("l", "sig.lfo"), ("t", "img.transform")], &[]);
        let t = n.node_by_name("t").unwrap().id;
        let l = n.node_by_name("l").unwrap().id;
        let set = |n: &mut Network, p: Param| {
            n.apply(
                &registry,
                lsr_core::graph::Edit::SetParam {
                    node: t,
                    param: "rotate".into(),
                    value: Some(p),
                },
            )
            .unwrap();
        };
        let ok = |n: &Network, extra: &str| prompt(extra).judge(n, &registry, &[], None).valid();
        let want = "exprs = [{type = \"img.transform\", param = \"rotate\", contains = \"time\"}]";
        assert!(!ok(&n, want));
        set(
            &mut n,
            Param {
                value: 0.into(),
                expr: Some("30 * time".into()),
                bind: None,
            },
        );
        assert!(ok(&n, want));
        assert!(!ok(
            &n,
            "exprs = [{type = \"img.transform\", param = \"rotate\", bind = true}]"
        ));
        set(&mut n, Param::bound(0.into(), l, "lfo"));
        assert!(ok(
            &n,
            "exprs = [{type = \"img.transform\", param = \"rotate\", bind = true, from = \"sig.lfo\"}]"
        ));
        assert!(!ok(
            &n,
            "exprs = [{type = \"img.transform\", param = \"rotate\", bind = true, from = \"sig.noise\"}]"
        ));
        assert!(ok(&n, "exprs = [{type = \"img.transform\"}]"));
    }
}
