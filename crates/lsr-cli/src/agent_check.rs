//! The network-generation eval's checker (`lasair agent eval`, M5 AC1): the
//! prompt file format and the scoring of the network the assistant left.
//! Pure functions of a network, so the reference solutions can be scored
//! without a model (see the tests).
//!
//! Every check is one point; a run is *valid* when it scores them all.
//!
//! - `min_nodes` (always one point), `max_nodes` (optional).
//! - `require`: one point per entry; some node's type matches it.
//! - `paths`: one point per entry, `"img.noise > img.feedback > cmp.window"`;
//!   following wires (any number of other nodes between), a path visits
//!   nodes whose types match each pattern, in that order. Channel
//!   bindings are not wires. A feedback loop may close a path (the same
//!   node can match two steps).
//! - `params`: one point per entry, `{type, name, eq | any_of | min | max |
//!   contains}`; some node whose type matches `type` has the parameter
//!   `name` with a *stored* value meeting every constraint (the schema
//!   default when the node does not override it; for a parameter driven by
//!   an expression, the expression's last result is what is stored, so test
//!   those with `exprs`). `eq` compares numbers with a small tolerance,
//!   strings and toggles exactly, and vectors and colours component by
//!   component (`component = i` tests only component `i` of a vector);
//!   `any_of` is a list of scalar alternatives for `eq`; `min`
//!   and `max` bound a number, or every component of a vector; `contains`
//!   is a case-insensitive substring of a string value.
//! - `exprs`: one point per entry, `{type, param?, contains?, bind?, from?,
//!   time_ok?, range?, period?}`; some node of that type has `param` (any
//!   parameter when omitted) driven: with `contains`, by an expression whose
//!   text contains it; with `bind = true`, by a channel binding; with
//!   neither, by either.
//!   - `from`: an expression must reference `op('<name>')` of a node whose
//!     type matches; a binding must come from such a node, or any SIG node
//!     upstream of the bound node must match (a Math fed by `sig.noise`
//!     satisfies `from = "sig.noise"`). `contains` and `from` both hold.
//!   - `time_ok = true`: the drive must move with time: an expression with
//!     `time` in it or reading a SIG node, or a binding to a SIG node.
//!   - `range = [lo, hi]`: the bound channel swings between those values
//!     (within 5%): an LFO's or Noise's offset ∓ amplitude, mapped through
//!     every Math in between (`from_min`..`from_max` to `to_min`..`to_max`,
//!     linear, on input `a`).
//!   - `period = s`: what drives it repeats every `s` seconds: a Timer's
//!     `length` or an LFO's 1 / `frequency`, bound or read with `op('<name>')`
//!     in an expression (or upstream of a Math), or an expression with `s`
//!     as a modulus: `% 12`, `fmod(x, 12)`, or `/ 12` next to a `%`.
//! - `either`: one point per entry, `{label, any = [{params?, exprs?, paths?,
//!   count?}, ...]}`: some alternative has every one of its checks hold (a
//!   right answer built another way, e.g. a flicker on a Level's brightness
//!   or on an HSV Adjust's value).
//! - `wired`: one point per entry, `{type, title_contains, from}`: some node
//!   of that type whose name or `title` contains the text (case-insensitive)
//!   has a node of type `from` upstream of it (a second output window that
//!   shows the same picture).
//! - `count`: one point per entry, `{type, min?, max?}`: the number of nodes
//!   of that type lies in the range.
//! - `seed`: a project directory the run starts from (a copy), for debug and
//!   edit tasks; `keep`: one point per node name that must still exist.
//! - `animated`: something outside the channel nodes moves.
//! - Always one point: every node cooks without an error.
//!
//! A model error makes the run invalid (and is named in the reasons) but
//! adds nothing to the points possible, so every model faces the same
//! maximum.
//!
//! Every type pattern above (in `require`, a path step, `type`, `from`) is
//! an exact type id (`img.render` does not match `img.render_points`), or a
//! family prefix when it ends in a dot (`sig.`). It may list alternatives
//! with `|`: `"pnt.noise|pnt.turbulence"` matches either, for prompts a
//! model can rightly answer more than one way.

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
    /// Right answers built another way: one point each, see the module docs.
    #[serde(default)]
    pub either: Vec<EitherCheck>,
    #[serde(default)]
    pub wired: Vec<WiredCheck>,
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
    /// Test only this component of a vector or colour.
    #[serde(default)]
    pub component: Option<usize>,
}

/// One point: some alternative holds all of its checks.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EitherCheck {
    pub label: String,
    pub any: Vec<Group>,
}

/// A set of checks that all hold together (one alternative of an `either`).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Group {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub params: Vec<ParamCheck>,
    #[serde(default)]
    pub exprs: Vec<ExprCheck>,
    #[serde(default)]
    pub count: Vec<CountCheck>,
}

/// A node of `type` named, or titled, with `title_contains` that has a node
/// of type `from` upstream of it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WiredCheck {
    #[serde(rename = "type")]
    pub type_id: String,
    pub title_contains: String,
    pub from: String,
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
    /// The drive moves with time: an expression using `time` or reading a
    /// SIG node, or a binding to a SIG node.
    #[serde(default)]
    pub time_ok: bool,
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
    /// The assistant itself failed: the run is invalid, the maximum unchanged.
    pub model_error: bool,
}

impl Verdict {
    pub fn valid(&self) -> bool {
        self.points == self.max && !self.model_error
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
            model_error: false,
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
                param_check_ok(net, registry, c),
                format!("no {}.{} {}", c.type_id, c.name, c.describe()),
            );
        }
        for c in &self.exprs {
            check(
                expr_check_ok(net, registry, c),
                format!("{} {} not driven as asked", c.type_id, c.describe()),
            );
        }
        for c in &self.count {
            let n = net.nodes().filter(|n| starts(n, &c.type_id)).count();
            check(
                count_ok(net, c),
                format!(
                    "{n} x {} (want {}..{})",
                    c.type_id,
                    c.min,
                    c.max.map_or("any".into(), |m| m.to_string())
                ),
            );
        }
        for e in &self.either {
            check(
                e.any.iter().any(|g| group_ok(net, registry, g)),
                format!("not met in any accepted way: {}", e.label),
            );
        }
        for w in &self.wired {
            check(
                wired_ok(net, w),
                format!(
                    "no {} titled {:?} fed by {}",
                    w.type_id, w.title_contains, w.from
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
            v.model_error = true;
            v.why.push(format!("model error: {e}"));
        }
        v
    }
}

fn param_check_ok(net: &Network, registry: &Registry, c: &ParamCheck) -> bool {
    net.nodes()
        .filter(|n| type_matches(&n.type_id, &c.type_id))
        .any(|n| param_ok(net, registry, n, c))
}

fn expr_check_ok(net: &Network, registry: &Registry, c: &ExprCheck) -> bool {
    net.nodes()
        .filter(|n| type_matches(&n.type_id, &c.type_id))
        .any(|n| expr_ok(net, registry, n, c))
}

fn count_ok(net: &Network, c: &CountCheck) -> bool {
    let n = net
        .nodes()
        .filter(|n| type_matches(&n.type_id, &c.type_id))
        .count();
    n >= c.min && c.max.is_none_or(|m| n <= m)
}

fn group_ok(net: &Network, registry: &Registry, g: &Group) -> bool {
    g.paths.iter().all(|p| path_exists(net, p))
        && g.params.iter().all(|c| param_check_ok(net, registry, c))
        && g.exprs.iter().all(|c| expr_check_ok(net, registry, c))
        && g.count.iter().all(|c| count_ok(net, c))
}

/// Every node upstream of `id` (through wires and channel bindings).
fn upstream_nodes(net: &Network, id: NodeId) -> Vec<NodeId> {
    let mut seen = vec![id];
    let mut k = 0;
    while k < seen.len() {
        let at = seen[k];
        let mut next: Vec<NodeId> = net
            .wires()
            .filter(|w| w.to.node == at)
            .map(|w| w.from.node)
            .collect();
        next.extend(net.bindings_of(at).map(|b| b.node));
        for n in next {
            if !seen.contains(&n) {
                seen.push(n);
            }
        }
        k += 1;
    }
    seen
}

fn wired_ok(net: &Network, w: &WiredCheck) -> bool {
    let want = w.title_contains.to_lowercase();
    net.nodes()
        .filter(|n| type_matches(&n.type_id, &w.type_id))
        .filter(|n| {
            n.name.to_lowercase().contains(&want)
                || n.params
                    .get("title")
                    .and_then(|p| p.value.as_str())
                    .is_some_and(|t| t.to_lowercase().contains(&want))
        })
        .any(|n| {
            upstream_nodes(net, n.id)
                .into_iter()
                .filter(|id| *id != n.id)
                .filter_map(|id| net.node(id))
                .any(|u| type_matches(&u.type_id, &w.from))
        })
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
        if let Some(i) = self.component {
            s.push(format!("(component {i})"));
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
        if self.time_ok {
            s += " moving with time";
        }
        s
    }
}

/// `t` is `pat` exactly, or one of its `|` alternatives; a pattern ending
/// in a dot is a family prefix (`sig.`).
fn type_matches(t: &str, pat: &str) -> bool {
    pat.split('|').any(|p| {
        let p = p.trim();
        if p.ends_with('.') {
            t.starts_with(p)
        } else {
            t == p
        }
    })
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
    let v = match c.component {
        Some(i) => match v.as_array().and_then(|a| a.get(i)) {
            Some(x) => x,
            None => return false,
        },
        None => v,
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

/// The SIG nodes feeding `src` through wires, `src` first (a Math remap fed
/// by a Timer repeats with the Timer).
fn sig_upstream(net: &Network, src: NodeId) -> Vec<NodeId> {
    let mut seen = vec![src];
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
}

/// The node names an expression reads with `op('name')` or `op("name")`.
fn op_refs(expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = expr;
    while let Some(i) = rest.find("op(") {
        rest = &rest[i + 3..];
        let inner = rest.trim_start();
        if let Some(q) = inner.chars().next().filter(|c| *c == '\'' || *c == '"')
            && let Some(end) = inner[1..].find(q)
        {
            out.push(inner[1..1 + end].to_owned());
        }
    }
    out
}

/// `expr` has `t` as a modulus: `% 12`, `fmod(x, 12)`, or `/ 12` beside a `%`.
fn modulo_form(expr: &str, t: f64) -> bool {
    let e: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let mut forms = vec![format!("{t}")];
    if t.fract() == 0.0 {
        forms.push(format!("{t:.1}"));
    }
    // `n` stands as a whole number at `at` (not the start of `120`).
    let whole = |at: usize, n: &str| {
        !e[at + n.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '.')
    };
    forms.iter().any(|n| {
        e.match_indices(&format!("%{n}"))
            .any(|(i, _)| whole(i + 1, n))
            || (e.contains("fmod(") && e.match_indices(&format!(",{n})")).next().is_some())
            || (e.contains('%')
                && e.match_indices(&format!("/{n}"))
                    .any(|(i, _)| whole(i + 1, n)))
    })
}

/// The values a channel node swings between: an LFO's or Noise's offset ∓
/// amplitude, mapped through each Math (input `a`) on the way.
fn swing_of(net: &Network, registry: &Registry, id: NodeId, depth: u32) -> Option<(f64, f64)> {
    let n = net.node(id)?;
    let num = |name: &str| net.param_value(registry, id, name).and_then(Value::as_f64);
    if n.type_id == "sig.lfo" || n.type_id == "sig.noise" {
        let (o, a) = (num("offset")?, num("amplitude")?);
        return Some((o - a.abs(), o + a.abs()));
    }
    if n.type_id == "sig.math" && depth < 8 {
        let (_, from) = net.inputs_of(id).find(|(name, _)| *name == "a")?;
        let (lo, hi) = swing_of(net, registry, from.node, depth + 1)?;
        let (f0, f1) = (num("from_min")?, num("from_max")?);
        let (t0, t1) = (num("to_min")?, num("to_max")?);
        if (f1 - f0).abs() < 1e-12 {
            return None;
        }
        let map = |x: f64| t0 + (x - f0) * (t1 - t0) / (f1 - f0);
        return Some((map(lo), map(hi)));
    }
    None
}

fn driven_ok(net: &Network, registry: &Registry, p: &Param, c: &ExprCheck) -> bool {
    let source = p.bind.as_ref().and_then(|b| net.node(b.node));
    let num = |n: &Node, name: &str| {
        net.param_value(registry, n.id, name)
            .and_then(Value::as_f64)
    };
    let close = |a: f64, b: f64| (a - b).abs() <= 0.05 * b.abs().max(1e-3);
    let is_sig = |n: &Node| n.type_id.starts_with("sig.");
    // The nodes an expression reads.
    let refs: Vec<&Node> = p
        .expr
        .as_deref()
        .map(op_refs)
        .unwrap_or_default()
        .iter()
        .filter_map(|name| net.node_by_name(name))
        .collect();
    let expr_ok = p.expr.as_deref().is_some_and(|e| {
        c.contains.as_ref().is_none_or(|s| e.contains(s.as_str()))
            && (!c.time_ok || e.contains("time") || refs.iter().any(|n| is_sig(n)))
            && c.from
                .as_ref()
                .is_none_or(|f| refs.iter().any(|n| type_matches(&n.type_id, f)))
    });
    let bind_ok = c.contains.is_none()
        && source.is_some_and(|src| {
            (!c.time_ok || is_sig(src))
                && c.from.as_ref().is_none_or(|f| {
                    sig_upstream(net, src.id)
                        .into_iter()
                        .filter_map(|id| net.node(id))
                        .any(|n| type_matches(&n.type_id, f))
                })
        });
    let ok = if c.bind { bind_ok } else { expr_ok || bind_ok };
    let in_range = c.range.is_none_or(|[lo, hi]| {
        source
            .and_then(|src| swing_of(net, registry, src.id, 0))
            .is_some_and(|(a, b)| close(a.min(b), lo) && close(a.max(b), hi))
    });
    // A Timer's length, an LFO's period.
    let period_of = |t: f64, n: &Node| {
        if n.type_id.starts_with("sig.timer") {
            num(n, "length").is_some_and(|l| close(l, t))
        } else if n.type_id.starts_with("sig.lfo") {
            num(n, "frequency").is_some_and(|f| f > 0.0 && close(1.0 / f, t))
        } else {
            false
        }
    };
    let repeats = c.period.is_none_or(|t| {
        let repeating = |start: &Node| {
            sig_upstream(net, start.id)
                .into_iter()
                .filter_map(|id| net.node(id))
                .any(|n| period_of(t, n))
        };
        source.is_some_and(repeating)
            || refs.iter().any(|n| repeating(n))
            || p.expr.as_deref().is_some_and(|e| modulo_form(e, t))
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
    use serde_json::json;

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
        // Tests run in parallel: one scratch folder per call.
        static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "lsr-check-{}-{}",
            std::process::id(),
            CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
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
        assert_eq!((v.points, v.max), (1, 5));
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

    // Test helpers: set a parameter on a node by name.

    fn set(n: &mut Network, node: &str, param: &str, p: Param) {
        let registry = crate::project::library().0;
        let id = n.node_by_name(node).expect(node).id;
        n.apply(
            &registry,
            lsr_core::graph::Edit::SetParam {
                node: id,
                param: param.into(),
                value: Some(p),
            },
        )
        .unwrap();
    }

    fn val(n: &mut Network, node: &str, param: &str, v: Value) {
        set(n, node, param, Param::constant(v));
    }

    fn expr(n: &mut Network, node: &str, param: &str, e: &str) {
        set(
            n,
            node,
            param,
            Param {
                value: 0.into(),
                expr: Some(e.into()),
                bind: None,
            },
        );
    }

    fn bind(n: &mut Network, node: &str, param: &str, src: &str) {
        let registry = crate::project::library().0;
        let id = n.node_by_name(src).unwrap().id;
        let at = n.node_by_name(node).unwrap().id;
        let fallback = n.param_value(&registry, at, param).cloned().unwrap();
        set(n, node, param, Param::bound(fallback, id, "out"));
    }

    /// The reasons the prompt `id` of the hard set gives for `n`.
    fn reasons(id: &str, n: &Network) -> Vec<String> {
        let registry = crate::project::library().0;
        let ps = prompts();
        let p = ps.iter().find(|p| p.id == id).expect(id);
        p.judge(n, &registry, &[], None).why
    }

    /// `want` (a check's label) is not among the reasons.
    fn met(id: &str, n: &Network, want: &str) -> bool {
        !reasons(id, n).iter().any(|w| w.contains(want))
    }

    // Item 6: exact type ids.

    #[test]
    fn a_type_pattern_is_exact_unless_it_ends_in_a_dot() {
        assert!(type_matches("img.render", "img.render"));
        assert!(!type_matches("img.render_points", "img.render"));
        assert!(type_matches(
            "img.render_points",
            "img.render|img.render_points"
        ));
        assert!(type_matches("sig.lfo", "sig."));
        assert!(!type_matches("img.noise", "sig."));
        let registry = crate::project::library().0;
        let n = net(&[("p", "img.render_points")], &[]);
        assert!(
            !prompt("require = [\"img.render\"]")
                .judge(&n, &registry, &[], None)
                .valid()
        );
        assert!(
            prompt("require = [\"img.render_points\"]")
                .judge(&n, &registry, &[], None)
                .valid()
        );
    }

    #[test]
    fn the_easy_set_still_means_what_it_did() {
        let text =
            std::fs::read_to_string(root().join("docs/milestones/m5/eval-prompts.toml")).unwrap();
        let ps = toml_edit::de::from_str::<Prompts>(&text).unwrap().prompt;
        let registry = crate::project::library().0;
        // Every pattern names a real type or a family: none can never match.
        for p in &ps {
            for r in &p.require {
                assert!(
                    r.split('|')
                        .all(|alt| alt.ends_with('.') || registry.get(alt.trim()).is_some()),
                    "{}: {r} names no operator",
                    p.id
                );
            }
        }
        // A points renderer still answers "render points".
        let n = net(&[("p", "pnt.particles"), ("r", "img.render_points")], &[]);
        let particles = ps.iter().find(|p| p.id == "particles").unwrap();
        assert!(particles.judge(&n, &registry, &[], None).valid());
        // A mesh render is img.render alone (the points renderer no longer counts).
        let mesh = ps.iter().find(|p| p.id == "mesh-sphere").unwrap();
        let wrong = net(&[("s", "msh.sphere"), ("r", "img.render_points")], &[]);
        assert!(!mesh.judge(&wrong, &registry, &[], None).valid());
        let right = net(&[("s", "msh.sphere"), ("r", "img.render")], &[]);
        assert!(mesh.judge(&right, &registry, &[], None).valid());
    }

    // Item 7: `from` follows op() references and remaps.

    #[test]
    fn from_needs_the_named_node_in_an_expression_or_upstream_of_a_binding() {
        let registry = crate::project::library().0;
        let ok = |n: &Network, extra: &str| prompt(extra).judge(n, &registry, &[], None).valid();
        let from_noise =
            "exprs = [{type = \"img.hsv_adjust\", param = \"hue\", from = \"sig.noise\"}]";
        let mut n = net(
            &[
                ("w", "sig.noise"),
                ("m", "sig.math"),
                ("l", "sig.lfo"),
                ("h", "img.hsv_adjust"),
            ],
            &[(0, 1, "a")],
        );
        // An expression that reads no node does not satisfy `from`.
        expr(&mut n, "h", "hue", "time * 20");
        assert!(!ok(&n, from_noise));
        // One that reads the wrong kind of node does not either.
        expr(&mut n, "h", "hue", "100 * op('l')['lfo']");
        assert!(!ok(&n, from_noise));
        expr(&mut n, "h", "hue", "100 * op('w')['chan1']");
        assert!(ok(&n, from_noise));
        // Double quotes too; and `contains` must hold as well.
        expr(&mut n, "h", "hue", "100 * op(\"w\")['chan1']");
        assert!(ok(&n, from_noise));
        assert!(!ok(
            &n,
            "exprs = [{type = \"img.hsv_adjust\", param = \"hue\", from = \"sig.noise\", contains = \"zzz\"}]"
        ));
        assert!(ok(
            &n,
            "exprs = [{type = \"img.hsv_adjust\", param = \"hue\", from = \"sig.noise\", contains = \"chan1\"}]"
        ));
        // A binding to the Math fed by the noise: noise is upstream.
        bind(&mut n, "h", "hue", "m");
        assert!(ok(&n, from_noise));
        // A binding to a Math that nothing feeds is not from the noise.
        let mut lone = net(&[("m", "sig.math"), ("h", "img.hsv_adjust")], &[]);
        bind(&mut lone, "h", "hue", "m");
        assert!(!ok(&lone, from_noise));
        assert!(ok(
            &lone,
            "exprs = [{type = \"img.hsv_adjust\", param = \"hue\", from = \"sig.math\"}]"
        ));
    }

    // Item 8: time_ok.

    #[test]
    fn time_ok_accepts_a_time_expression_or_a_signal_binding() {
        let registry = crate::project::library().0;
        let ok = |n: &Network| {
            prompt("exprs = [{type = \"img.render\", param = \"azimuth\", time_ok = true}]")
                .judge(n, &registry, &[], None)
                .valid()
        };
        let mut n = net(&[("l", "sig.lfo"), ("r", "img.render")], &[]);
        assert!(!ok(&n), "a constant is not motion");
        expr(&mut n, "r", "azimuth", "time * 6");
        assert!(ok(&n));
        expr(&mut n, "r", "azimuth", "90");
        assert!(!ok(&n), "an expression with no time in it");
        expr(&mut n, "r", "azimuth", "360 * op('l')['lfo']");
        assert!(ok(&n), "reading a signal moves");
        bind(&mut n, "r", "azimuth", "l");
        assert!(ok(&n), "a binding to a signal moves");
    }

    // Item 9: range through a Math remap.

    #[test]
    fn range_maps_the_swing_through_math() {
        let registry = crate::project::library().0;
        let want = "exprs = [{type = \"img.transform\", param = \"scale\", bind = true, range = [0.5, 2]}]";
        let ok = |n: &Network| prompt(want).judge(n, &registry, &[], None).valid();
        // An LFO bound directly: offset 1.25 +- 0.75.
        let mut n = net(&[("l", "sig.lfo"), ("t", "img.transform")], &[]);
        val(&mut n, "l", "offset", json!(1.25));
        val(&mut n, "l", "amplitude", json!(0.75));
        bind(&mut n, "t", "scale", "l");
        assert!(ok(&n));
        val(&mut n, "l", "amplitude", json!(0.5));
        assert!(!ok(&n));
        // Through a Math: the LFO's -1..1 mapped -1..1 -> 0.5..2.
        let mut m = net(
            &[("l", "sig.lfo"), ("m", "sig.math"), ("t", "img.transform")],
            &[(0, 1, "a")],
        );
        val(&mut m, "m", "from_min", json!(-1.0));
        val(&mut m, "m", "from_max", json!(1.0));
        val(&mut m, "m", "to_min", json!(0.5));
        val(&mut m, "m", "to_max", json!(2.0));
        bind(&mut m, "t", "scale", "m");
        assert!(ok(&m));
        // The same to_min..to_max with the wrong from range is a different swing.
        val(&mut m, "m", "from_min", json!(0.0));
        assert!(!ok(&m), "the LFO's -1..1 now maps to 0.5-0.75..2+0.75");
        // And the remap reads the LFO's own swing (amplitude 2: -2..2 over 0..1 is wider).
        val(&mut m, "m", "from_min", json!(-2.0));
        val(&mut m, "m", "from_max", json!(2.0));
        val(&mut m, "l", "amplitude", json!(2.0));
        assert!(ok(&m), "-2..2 mapped over -2..2 is the full to range");
        // A Math with nothing wired into it does not swing.
        let mut lone = net(&[("m", "sig.math"), ("t", "img.transform")], &[]);
        val(&mut lone, "m", "to_min", json!(0.5));
        val(&mut lone, "m", "to_max", json!(2.0));
        bind(&mut lone, "t", "scale", "m");
        assert!(!ok(&lone));
    }

    // Item 10: periods.

    #[test]
    fn period_follows_op_references_and_needs_a_modulus() {
        let registry = crate::project::library().0;
        let ok = |n: &Network| {
            prompt("exprs = [{type = \"img.switch\", param = \"index\", period = 12}]")
                .judge(n, &registry, &[], None)
                .valid()
        };
        let mut n = net(&[("t", "sig.timer"), ("s", "img.switch")], &[]);
        val(&mut n, "t", "length", json!(12.0));
        // A plain expression: the period must be a modulus, not any 12.
        expr(&mut n, "s", "index", "sin(time * 12)");
        assert!(!ok(&n), "sin(time*12) repeats every 0.52 s");
        expr(&mut n, "s", "index", "time % 12");
        assert!(ok(&n));
        expr(&mut n, "s", "index", "fmod(time, 12)");
        assert!(ok(&n));
        expr(&mut n, "s", "index", "3 * (time % 12) / 12");
        assert!(ok(&n));
        expr(&mut n, "s", "index", "time / 12");
        assert!(!ok(&n), "a division alone does not repeat");
        expr(&mut n, "s", "index", "time % 120");
        assert!(!ok(&n), "120 is not 12");
        // Reading the Timer: its length is the period.
        expr(&mut n, "s", "index", "3 * op('t')['fraction']");
        assert!(ok(&n));
        val(&mut n, "t", "length", json!(7.0));
        assert!(!ok(&n), "a 7 s timer");
        // An LFO read in an expression: 1 / frequency.
        let mut l = net(&[("l", "sig.lfo"), ("s", "img.switch")], &[]);
        val(&mut l, "l", "frequency", json!(1.0 / 12.0));
        expr(&mut l, "s", "index", "2 + op('l')['lfo']");
        assert!(ok(&l));
    }

    // Item 11: a model error.

    #[test]
    fn a_model_error_makes_the_run_invalid_without_changing_the_maximum() {
        let registry = crate::project::library().0;
        let n = net(&[("a", "img.noise")], &[]);
        let p = prompt("require = [\"img.noise\"]");
        let fine = p.judge(&n, &registry, &[], None);
        let broke = p.judge(&n, &registry, &[], Some("rate limit"));
        assert_eq!(fine.max, broke.max, "the same maximum for every model");
        assert_eq!(fine.points, broke.points);
        assert!(fine.valid() && !broke.valid());
        assert!(
            broke
                .why
                .iter()
                .any(|w| w.contains("model error: rate limit"))
        );
    }

    // The prompts' loosened checks: a right answer passes, a wrong one fails.

    #[test]
    fn h_fix_broken_chain_accepts_both_full_hd_forms_and_either_order() {
        let id = "h-fix-broken-chain";
        let hd = "full HD";
        let build = |mode: &str, setup: &dyn Fn(&mut Network)| {
            let mut n = net(
                &[
                    ("base_noise", "img.noise"),
                    ("blur_soft", "img.blur"),
                    ("shake", "img.transform"),
                    ("res", "img.resolution"),
                    ("glow", "img.bloom"),
                    ("main_out", "cmp.window"),
                ],
                &[
                    (0, 1, "in"),
                    (1, 2, "in"),
                    (2, 3, "in"),
                    (3, 4, "in"),
                    (4, 5, "in"),
                ],
            );
            val(&mut n, "res", "mode", json!(mode));
            setup(&mut n);
            n
        };
        let pixels = build("pixels", &|n| {
            val(n, "res", "width", json!(1920));
            val(n, "res", "height", json!(1080));
        });
        assert!(met(id, &pixels, hd));
        let fraction = build("fraction", &|n| val(n, "res", "fraction", json!(1.5)));
        assert!(met(id, &fraction, hd));
        // Still tiny, or the wrong fraction.
        let tiny = build("pixels", &|n| {
            val(n, "res", "width", json!(160));
            val(n, "res", "height", json!(90));
        });
        assert!(!met(id, &tiny, hd));
        let half = build("fraction", &|_| {});
        assert!(!met(id, &half, hd), "the default 0.5 is not full HD");
        // Blur and transform in either order: both are on a path to the window.
        let blur_last = net(
            &[
                ("base_noise", "img.noise"),
                ("shake", "img.transform"),
                ("blur_soft", "img.blur"),
                ("main_out", "cmp.window"),
            ],
            &[(0, 1, "in"), (1, 2, "in"), (2, 3, "in")],
        );
        let r = reasons(id, &blur_last);
        assert!(
            !r.iter().any(|w| w.contains("no path img.noise > img.blur")),
            "{r:?}"
        );
        assert!(
            !r.iter()
                .any(|w| w.contains("no path img.noise > img.transform")),
            "{r:?}"
        );
        // The blur on a dead-end branch is not in the chain.
        let dead = net(
            &[
                ("base_noise", "img.noise"),
                ("blur_soft", "img.blur"),
                ("shake", "img.transform"),
                ("main_out", "cmp.window"),
            ],
            &[(0, 1, "in"), (0, 2, "in"), (2, 3, "in")],
        );
        assert!(
            reasons(id, &dead)
                .iter()
                .any(|w| w.contains("no path img.noise > img.blur"))
        );
        // `shake` (not named in the prompt) may be renamed.
        let ps = prompts();
        assert!(
            !ps.iter()
                .find(|p| p.id == id)
                .unwrap()
                .keep
                .contains(&"shake".to_owned())
        );
    }

    #[test]
    fn h_three_looks_accepts_a_spinning_transform() {
        let id = "h-three-looks";
        let spin = "the shape spins";
        let mut by_shape = net(&[("s", "img.shape"), ("w", "img.switch")], &[(0, 1, "in1")]);
        assert!(!met(id, &by_shape, spin), "a still shape");
        expr(&mut by_shape, "s", "rotate", "time * 30");
        assert!(met(id, &by_shape, spin));
        let mut by_transform = net(
            &[
                ("s", "img.shape"),
                ("t", "img.transform"),
                ("w", "img.switch"),
            ],
            &[(0, 1, "in"), (1, 2, "in1")],
        );
        assert!(!met(id, &by_transform, spin));
        expr(&mut by_transform, "t", "rotate", "time * 30");
        assert!(met(id, &by_transform, spin));
        // A transform that is not on the shape's chain does not spin it.
        let mut elsewhere = net(
            &[
                ("s", "img.shape"),
                ("t", "img.transform"),
                ("w", "img.switch"),
            ],
            &[(0, 2, "in1")],
        );
        expr(&mut elsewhere, "t", "rotate", "time * 30");
        assert!(!met(id, &elsewhere, spin));
        // The 12 s loop: a modulus, not sin(time * 12).
        let mut sw = net(&[("w", "img.switch")], &[]);
        expr(&mut sw, "w", "index", "sin(time * 12)");
        assert!(reasons(id, &sw).iter().any(|w| w.contains("img.switch")));
        expr(&mut sw, "w", "index", "3 * (time % 12) / 12");
        assert!(
            !reasons(id, &sw)
                .iter()
                .any(|w| w.contains("img.switch index"))
        );
    }

    #[test]
    fn h_tunnel_two_screens_accepts_a_level_or_hsv_fade_and_either_colour_order() {
        let id = "h-tunnel-two-screens";
        let fade = "the trails fade";
        let mut n = net(&[("l", "img.level"), ("h", "img.hsv_adjust")], &[]);
        assert!(!met(id, &n, fade), "nothing dims");
        val(&mut n, "l", "brightness", json!(0.95));
        assert!(met(id, &n, fade));
        let mut h = net(&[("l", "img.level"), ("h", "img.hsv_adjust")], &[]);
        val(&mut h, "h", "value", json!(0.95));
        assert!(met(id, &h, fade));
        // hsv and bloom in either order after the loop.
        let chain = |order: [&str; 2]| {
            net(
                &[
                    ("noise", "img.noise"),
                    ("fold", "img.kaleidoscope"),
                    ("acc", "img.composite"),
                    ("x", order[0]),
                    ("y", order[1]),
                    ("w", "cmp.window"),
                ],
                &[
                    (0, 1, "in"),
                    (1, 2, "a"),
                    (2, 3, "in"),
                    (3, 4, "in"),
                    (4, 5, "in"),
                ],
            )
        };
        for order in [
            ["img.hsv_adjust", "img.bloom"],
            ["img.bloom", "img.hsv_adjust"],
        ] {
            let r = reasons(id, &chain(order));
            for bad in ["img.hsv_adjust > cmp.window", "img.bloom > cmp.window"] {
                assert!(!r.iter().any(|w| w.contains(bad)), "{order:?}: {r:?}");
            }
        }
        // A fade node that is not in the loop does not count as the loop.
        let r = reasons(id, &chain(["img.hsv_adjust", "img.bloom"]));
        assert!(
            r.iter()
                .any(|w| w.contains("img.feedback > img.level|img.hsv_adjust")),
            "{r:?}"
        );
    }

    #[test]
    fn h_layered_stage_accepts_one_mixer_or_two_composites() {
        let id = "h-layered-stage";
        let layers = "the three layers combined";
        let composites = || {
            let mut n = net(
                &[
                    ("floor", "img.ramp"),
                    ("pent", "img.shape"),
                    ("grid", "img.tile"),
                    ("lit", "img.composite"),
                    ("mist", "img.noise"),
                    ("shade", "img.composite"),
                    ("w", "cmp.window"),
                ],
                &[
                    (1, 2, "in"),
                    (2, 3, "a"),
                    (0, 3, "b"),
                    (3, 5, "a"),
                    (4, 5, "b"),
                    (5, 6, "in"),
                ],
            );
            val(&mut n, "lit", "operation", json!("add"));
            val(&mut n, "shade", "operation", json!("multiply"));
            n
        };
        assert!(met(id, &composites(), layers));
        let mut wrong = composites();
        val(&mut wrong, "shade", "operation", json!("add"));
        assert!(!met(id, &wrong, layers));
        let mut mixer = net(
            &[
                ("floor", "img.ramp"),
                ("pent", "img.shape"),
                ("grid", "img.tile"),
                ("mist", "img.noise"),
                ("deck", "img.mixer"),
                ("w", "cmp.window"),
            ],
            &[
                (1, 2, "in"),
                (0, 4, "layer0"),
                (2, 4, "layer1"),
                (3, 4, "layer2"),
                (4, 5, "in"),
            ],
        );
        assert!(!met(id, &mixer, layers), "all layers still blend over");
        val(&mut mixer, "deck", "blend1", json!("add"));
        val(&mut mixer, "deck", "blend2", json!("multiply"));
        assert!(met(id, &mixer, layers));
        // Six nodes are enough now.
        assert!(!reasons(id, &mixer).iter().any(|w| w.contains("nodes <")));
    }

    #[test]
    fn h_torus_dust_takes_the_colour_before_or_after_the_turbulence() {
        let id = "h-torus-dust";
        let chain = |order: [(&str, &str); 2]| {
            let mut n = net(
                &[
                    ("ring", "msh.torus"),
                    ("dust", "msh.scatter"),
                    (order[0].0, order[0].1),
                    (order[1].0, order[1].1),
                    ("dots", "mat.point_sprite"),
                    ("cam", "img.render"),
                ],
                &[
                    (0, 1, "in"),
                    (1, 2, "in"),
                    (2, 3, "in"),
                    (3, 5, "geo"),
                    (4, 5, "mat"),
                ],
            );
            expr(&mut n, "turb", "evolution", "time * 0.3");
            expr(&mut n, "cam", "azimuth", "time * 12");
            n
        };
        for order in [
            [("turb", "pnt.turbulence"), ("heights", "pnt.colour")],
            [("heights", "pnt.colour"), ("turb", "pnt.turbulence")],
        ] {
            let r = reasons(id, &chain(order));
            assert!(
                !r.iter()
                    .any(|w| w.contains("no path") || w.contains("not driven")),
                "{order:?}: {r:?}"
            );
        }
        // A camera that never moves fails.
        let mut bound = chain([("turb", "pnt.turbulence"), ("heights", "pnt.colour")]);
        expr(&mut bound, "cam", "azimuth", "90");
        assert!(
            reasons(id, &bound).iter().any(|w| w.contains("azimuth")),
            "a fixed camera fails"
        );
        // The turbulence that never reaches the render fails the path.
        let mut off = chain([("turb", "pnt.turbulence"), ("heights", "pnt.colour")]);
        let reg = crate::project::library().0;
        let heights = off.node_by_name("heights").unwrap().id;
        let cam = off.node_by_name("cam").unwrap().id;
        off.apply(
            &reg,
            lsr_core::graph::Edit::Disconnect {
                wire: lsr_core::graph::Wire::new(
                    lsr_core::graph::Endpoint::new(heights, "out"),
                    lsr_core::graph::Endpoint::new(cam, "geo"),
                ),
            },
        )
        .unwrap();
        assert!(reasons(id, &off).iter().any(|w| w.contains("no path")));
    }

    #[test]
    fn h_spark_fountain_wants_upward_velocity_and_gravity_pulling_down() {
        let id = "h-spark-fountain";
        let mut n = net(&[("p", "pnt.particles")], &[]);
        let r = reasons(id, &n);
        assert!(
            r.iter().any(|w| w.contains("velocity")),
            "the default throws nothing: {r:?}"
        );
        assert!(
            r.iter().any(|w| w.contains("gravity")),
            "the default pulls nothing: {r:?}"
        );
        val(&mut n, "p", "velocity", json!([0.0, 1.5, 0.0]));
        val(&mut n, "p", "gravity", json!([0.0, -1.2, 0.0]));
        let r = reasons(id, &n);
        assert!(
            !r.iter()
                .any(|w| w.contains("velocity") || w.contains("gravity")),
            "{r:?}"
        );
        // Thrown down, pulled up: both wrong.
        val(&mut n, "p", "velocity", json!([0.0, -1.5, 0.0]));
        val(&mut n, "p", "gravity", json!([0.0, 1.2, 0.0]));
        let r = reasons(id, &n);
        assert!(
            r.iter().any(|w| w.contains("velocity")) && r.iter().any(|w| w.contains("gravity"))
        );
        // Sideways: only the y component counts.
        val(&mut n, "p", "velocity", json!([2.0, 0.0, 0.0]));
        assert!(reasons(id, &n).iter().any(|w| w.contains("velocity")));
    }

    #[test]
    fn h_breathing_clouds_takes_the_hue_from_noise_directly_or_through_a_remap() {
        let id = "h-breathing-clouds";
        let hue = "img.hsv_adjust hue bound from sig.noise";
        let direct = || {
            let mut n = net(&[("w", "sig.noise"), ("h", "img.hsv_adjust")], &[]);
            bind(&mut n, "h", "hue", "w");
            n
        };
        assert!(met(id, &direct(), hue));
        let mut remap = net(
            &[
                ("w", "sig.noise"),
                ("m", "sig.math"),
                ("h", "img.hsv_adjust"),
            ],
            &[(0, 1, "a")],
        );
        bind(&mut remap, "h", "hue", "m");
        assert!(met(id, &remap, hue));
        // A remap fed by the LFO is not the noise.
        let mut lfo = net(
            &[("l", "sig.lfo"), ("m", "sig.math"), ("h", "img.hsv_adjust")],
            &[(0, 1, "a")],
        );
        bind(&mut lfo, "h", "hue", "m");
        assert!(!met(id, &lfo, hue));
        // The breath: 0.5..2 every 4 s, through a remap of the LFO.
        let breath = |freq: f64| {
            let mut n = net(
                &[("l", "sig.lfo"), ("m", "sig.math"), ("t", "img.transform")],
                &[(0, 1, "a")],
            );
            val(&mut n, "l", "frequency", json!(freq));
            val(&mut n, "m", "from_min", json!(-1.0));
            val(&mut n, "m", "to_min", json!(0.5));
            val(&mut n, "m", "to_max", json!(2.0));
            bind(&mut n, "t", "scale", "m");
            n
        };
        assert!(met(id, &breath(0.25), "img.transform scale"));
        assert!(
            !met(id, &breath(1.0), "img.transform scale"),
            "a 1 s breath"
        );
    }

    #[test]
    fn h_cue_sheet_must_read_the_table_row_mid() {
        let id = "h-cue-sheet";
        let mut n = net(
            &[
                ("cues", "dat.table"),
                ("lights", "img.level"),
                ("clouds", "img.noise"),
            ],
            &[],
        );
        expr(&mut n, "lights", "brightness", "op('cues')['mid', 'level']");
        assert!(met(id, &n, "img.level brightness"));
        // A copied number, a table that is not read, another row, and a
        // lookalike name are all wrong.
        expr(&mut n, "lights", "brightness", "0.6");
        assert!(!met(id, &n, "img.level brightness"));
        expr(&mut n, "lights", "brightness", "op('cues')['dim', 'level']");
        assert!(!met(id, &n, "img.level brightness"));
        expr(
            &mut n,
            "lights",
            "brightness",
            "op('clouds')['mid', 'level']",
        );
        assert!(
            !met(id, &n, "img.level brightness"),
            "a node that is not a table"
        );
        // The clouds move by time or by a signal.
        assert!(reasons(id, &n).iter().any(|w| w.contains("img.noise")));
        expr(&mut n, "clouds", "z", "time * 0.3");
        assert!(!reasons(id, &n).iter().any(|w| w.contains("img.noise z")));
    }

    #[test]
    fn h_worn_tape_accepts_a_flicker_on_level_or_hsv() {
        let id = "h-worn-tape";
        let flicker = "the brightness flickers";
        let mut n = net(&[("l", "img.level"), ("h", "img.hsv_adjust")], &[]);
        assert!(!met(id, &n, flicker));
        expr(&mut n, "l", "brightness", "0.85 + 0.15 * sin(time * 37)");
        assert!(met(id, &n, flicker));
        let mut h = net(&[("l", "img.level"), ("h", "img.hsv_adjust")], &[]);
        expr(&mut h, "h", "value", "0.85 + 0.15 * sin(time * 37)");
        assert!(met(id, &h, flicker));
        // A flicker on something else (saturation) is not brightness.
        let mut s = net(&[("h", "img.hsv_adjust")], &[]);
        expr(&mut s, "h", "saturation", "0.5 + sin(time)");
        assert!(!met(id, &s, flicker));
    }

    #[test]
    fn h_stark_poster_takes_the_mirror_anywhere_in_the_chain() {
        let id = "h-stark-poster";
        let mirror = "no path img.noise > img.mirror";
        let after = net(
            &[
                ("n", "img.noise"),
                ("t", "img.threshold"),
                ("m", "img.mirror"),
                ("c", "img.lookup"),
            ],
            &[(0, 1, "in"), (1, 2, "in"), (2, 3, "in")],
        );
        assert!(met(id, &after, mirror));
        let last = net(
            &[
                ("n", "img.noise"),
                ("t", "img.threshold"),
                ("c", "img.lookup"),
                ("m", "img.mirror"),
            ],
            &[(0, 1, "in"), (1, 2, "in"), (2, 3, "in")],
        );
        assert!(met(id, &last, mirror));
        let none = net(
            &[
                ("n", "img.noise"),
                ("t", "img.threshold"),
                ("c", "img.lookup"),
            ],
            &[(0, 1, "in"), (1, 2, "in")],
        );
        assert!(!met(id, &none, mirror));
        // A mirror that nothing feeds is not in the chain.
        let loose = net(&[("n", "img.noise"), ("m", "img.mirror")], &[]);
        assert!(!met(id, &loose, mirror));
    }

    #[test]
    fn h_show_change_request_needs_a_sized_preview_fed_with_the_final_picture() {
        let id = "h-show-change-request";
        let size = "the Preview window is 480 x 270";
        let feed = "titled \"preview\"";
        let base = || {
            net(
                &[
                    ("title", "img.text"),
                    ("main", "cmp.window"),
                    ("Preview", "cmp.window"),
                ],
                &[(0, 1, "in"), (0, 2, "in")],
            )
        };
        let mut a = base();
        val(&mut a, "Preview", "title", json!("Preview"));
        assert!(!met(id, &a, size), "the default window is 1920 x 1080");
        assert!(met(id, &a, feed));
        val(&mut a, "Preview", "width", json!(480));
        val(&mut a, "Preview", "height", json!(270));
        assert!(met(id, &a, size));
        let mut b = base();
        val(&mut b, "Preview", "window_size", json!([480.0, 270.0]));
        assert!(met(id, &b, size));
        // A Preview window nothing feeds, or one fed from before the text,
        // does not show the final picture.
        let unfed = net(
            &[
                ("title", "img.text"),
                ("main", "cmp.window"),
                ("Preview", "cmp.window"),
            ],
            &[(0, 1, "in")],
        );
        assert!(!met(id, &unfed, feed));
        let early = net(
            &[
                ("n", "img.noise"),
                ("title", "img.text"),
                ("main", "cmp.window"),
                ("Preview", "cmp.window"),
            ],
            &[(0, 1, "in"), (1, 2, "in"), (0, 3, "in")],
        );
        assert!(!met(id, &early, feed));
        // Titled through the title parameter rather than the name.
        let mut titled = net(
            &[("title", "img.text"), ("small", "cmp.window")],
            &[(0, 1, "in")],
        );
        val(&mut titled, "small", "title", json!("My preview"));
        assert!(met(id, &titled, feed));
    }

    #[test]
    fn h_fix_trails_takes_the_fade_before_or_after_the_drift() {
        let id = "h-fix-trails";
        let build = |level_first: bool| {
            let (a, b) = if level_first {
                ("img.level", "img.transform")
            } else {
                ("img.transform", "img.level")
            };
            net(
                &[
                    ("mem", "img.feedback"),
                    ("x", a),
                    ("y", b),
                    ("acc", "img.composite"),
                    ("w", "cmp.window"),
                ],
                &[
                    (0, 1, "in"),
                    (1, 2, "in"),
                    (2, 3, "b"),
                    (3, 0, "loop"),
                    (3, 4, "in"),
                ],
            )
        };
        for first in [true, false] {
            let r = reasons(id, &build(first));
            assert!(
                !r.iter().any(|w| w.contains("no path img.feedback")),
                "{first}: {r:?}"
            );
        }
        // No loop wire, no trails.
        let open = net(
            &[
                ("mem", "img.feedback"),
                ("t", "img.transform"),
                ("l", "img.level"),
                ("acc", "img.composite"),
            ],
            &[(0, 1, "in"), (1, 2, "in"), (2, 3, "b")],
        );
        assert!(
            reasons(id, &open)
                .iter()
                .any(|w| w.contains("no path img.feedback"))
        );
    }

    #[test]
    fn h_orbit_cam_azimuth_may_follow_time_or_a_signal() {
        let id = "h-orbit-cam";
        let mut n = net(&[("l", "sig.lfo"), ("cam", "img.render")], &[]);
        assert!(!met(id, &n, "img.render azimuth"));
        bind(&mut n, "cam", "azimuth", "l");
        assert!(met(id, &n, "img.render azimuth"));
    }

    #[test]
    fn h_wire_the_signals_needs_the_signals_read_by_name() {
        let id = "h-wire-the-signals";
        let mut n = net(
            &[
                ("pulse", "sig.lfo"),
                ("wander", "sig.noise"),
                ("lvl", "img.level"),
                ("hue", "img.hsv_adjust"),
            ],
            &[],
        );
        bind(&mut n, "lvl", "brightness", "pulse");
        bind(&mut n, "hue", "hue", "wander");
        let r = reasons(id, &n);
        assert!(!r.iter().any(|w| w.contains("not driven")), "{r:?}");
        // An expression that never reads the signal is not wired to it.
        expr(&mut n, "lvl", "brightness", "0.5 + 0.5 * sin(time)");
        assert!(reasons(id, &n).iter().any(|w| w.contains("img.level")));
        expr(
            &mut n,
            "lvl",
            "brightness",
            "0.7 + 0.3 * op('pulse')['lfo']",
        );
        assert!(!reasons(id, &n).iter().any(|w| w.contains("img.level")));
        // The wrong signal.
        bind(&mut n, "hue", "hue", "pulse");
        assert!(reasons(id, &n).iter().any(|w| w.contains("img.hsv_adjust")));
    }
}
