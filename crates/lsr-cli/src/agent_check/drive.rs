//! Behavioural checks for the expert tier: what a driven parameter does over
//! time. The drive (a native expression, a Python tuple of native
//! expressions, or a channel binding) is evaluated at chosen times with the
//! engine's own expression evaluator, reading tables as they are typed and
//! SIG nodes by their formulas (LFO, Timer, Constant, Beat, Math, Merge).
//! A drive built from anything else (a Python expression, a Noise or Envelope
//! channel, a Select over a table) cannot be evaluated here: the check then
//! falls back to what the prompt names (`fallback`), see `agent_check.rs`.

use lsr_core::dat::{Dat, Table};
use lsr_core::expr::{CellAt, Env, EvalError, Expr, cell_number};
use lsr_core::graph::{Network, NodeId};
use lsr_core::registry::Registry;
use serde_json::Value;

/// Why a drive has no value at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fail {
    /// Built from something this evaluator does not model.
    Unsupported,
    /// Evaluated and wrong (an unknown cell, a division by zero, ...).
    Bad,
}

const MAX_DEPTH: u32 = 12;

/// A table's numbers are replaced by `x * 3 + 0.5` when `perturb` is on:
/// what the drive would give after the table is edited.
pub(super) struct Eval<'a> {
    pub net: &'a Network,
    pub registry: &'a Registry,
    pub t: f64,
    pub perturb: bool,
}

type Channels = Vec<(String, f64)>;

impl Eval<'_> {
    /// A number parameter as it is driven (a Beat's tempo may follow a table).
    fn num(&self, id: NodeId, name: &str, depth: u32) -> Option<f64> {
        self.drive(id, name, 0, depth + 1).ok()
    }

    fn text(&self, id: NodeId, name: &str) -> String {
        self.net
            .param_value(self.registry, id, name)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    /// Component `comp` of parameter `name` of node `id` as it is driven.
    pub fn drive(&self, id: NodeId, name: &str, comp: usize, depth: u32) -> Result<f64, Fail> {
        if depth > MAX_DEPTH {
            return Err(Fail::Unsupported);
        }
        let node = self.net.node(id).ok_or(Fail::Bad)?;
        let stored = self.net.param_value(self.registry, id, name);
        let width = stored
            .and_then(Value::as_array)
            .map_or(1, std::vec::Vec::len);
        if let Some(p) = node.params.get(name) {
            if let Some(b) = &p.bind {
                let chans = self.channels(b.node, depth + 1)?;
                let pick = if b.channel.is_empty() {
                    if chans.len() >= width {
                        chans.get(comp)
                    } else {
                        chans.first()
                    }
                } else {
                    chans.iter().find(|(n, _)| *n == b.channel)
                };
                return pick.map(|(_, v)| *v).ok_or(Fail::Bad);
            }
            if let Some(e) = &p.expr {
                return self.expression(id, e, comp, depth);
            }
        }
        match stored {
            Some(Value::Number(n)) => n.as_f64().ok_or(Fail::Bad),
            Some(Value::Array(a)) => a.get(comp).and_then(Value::as_f64).ok_or(Fail::Bad),
            _ => Err(Fail::Bad),
        }
    }

    fn expression(&self, id: NodeId, source: &str, comp: usize, depth: u32) -> Result<f64, Fail> {
        let parts = tuple_parts(source);
        let text = if parts.is_empty() {
            source
        } else {
            parts.get(comp).copied().ok_or(Fail::Bad)?
        };
        let expr = Expr::parse(text).map_err(|_| Fail::Unsupported)?;
        let env = ExprEnv {
            ev: self,
            id,
            refs: expr
                .refs()
                .iter()
                .map(|n| self.net.node_by_name(n).map(|n| n.id))
                .collect(),
            depth,
        };
        expr.eval(&env).map_err(|e| match e {
            EvalError::NotDat(_) => Fail::Unsupported,
            _ => Fail::Bad,
        })
    }

    /// The channels a SIG node outputs at `self.t`.
    fn channels(&self, id: NodeId, depth: u32) -> Result<Channels, Fail> {
        if depth > MAX_DEPTH {
            return Err(Fail::Unsupported);
        }
        let node = self.net.node(id).ok_or(Fail::Bad)?;
        let t = self.t;
        let off = |name: &str| self.text(id, name) == "off" || self.text(id, name).is_empty();
        let input = |name: &str| {
            self.net
                .inputs_of(id)
                .find(|(n, _)| *n == name)
                .map(|(_, e)| e.node)
        };
        match node.type_id.as_str() {
            "sig.lfo" => {
                if !off("sync") || input("clock").is_some() {
                    return Err(Fail::Unsupported);
                }
                let x = t * self.num(id, "frequency", depth).ok_or(Fail::Bad)?
                    + self.num(id, "phase", depth).ok_or(Fail::Bad)?;
                let v = self.num(id, "offset", depth).ok_or(Fail::Bad)?
                    + self.num(id, "amplitude", depth).ok_or(Fail::Bad)?
                        * wave(&self.text(id, "waveform"), x);
                Ok(vec![("lfo".into(), v)])
            }
            "sig.timer" => {
                if !off("sync") || input("clock").is_some() {
                    return Err(Fail::Unsupported);
                }
                let len = self.num(id, "length", depth).ok_or(Fail::Bad)?.max(1e-6);
                let looping =
                    self.net.param_value(self.registry, id, "loop") != Some(&false.into());
                let secs = if looping { t % len } else { t.min(len) };
                Ok(vec![
                    ("seconds".into(), secs),
                    ("fraction".into(), secs / len),
                    ("cycles".into(), (t / len).floor()),
                ])
            }
            "sig.constant" => Ok((1..=4)
                .filter_map(|i| {
                    let name = self.text(id, &format!("name{i}"));
                    (!name.is_empty()).then(|| {
                        (
                            name,
                            self.num(id, &format!("value{i}"), depth)
                                .unwrap_or_default(),
                        )
                    })
                })
                .collect()),
            "sig.beat" => {
                if self.text(id, "source") != "internal" || input("in").is_some() {
                    return Err(Fail::Unsupported);
                }
                let bpm = self.num(id, "bpm", depth).ok_or(Fail::Bad)?;
                let per_bar = self.num(id, "beats_per_bar", depth).unwrap_or(4.0).max(1.0);
                let beats = t * bpm / 60.0;
                Ok(vec![
                    ("beat".into(), beats.floor()),
                    ("beat_phase".into(), beats.fract()),
                    ("bar".into(), (beats / per_bar).floor()),
                    ("bar_phase".into(), (beats / per_bar).fract()),
                    ("bpm".into(), bpm),
                ])
            }
            "sig.merge" => {
                let mut out = Vec::new();
                for name in ["a", "b", "c", "d"] {
                    if let Some(src) = input(name) {
                        out.extend(self.channels(src, depth + 1)?);
                    }
                }
                Ok(out)
            }
            "sig.math" => {
                let a = self.channels(input("a").ok_or(Fail::Unsupported)?, depth + 1)?;
                let b = match input("b") {
                    Some(b) => Some(self.channels(b, depth + 1)?),
                    None => None,
                };
                let (f0, f1) = (
                    self.num(id, "from_min", depth).ok_or(Fail::Bad)?,
                    self.num(id, "from_max", depth).ok_or(Fail::Bad)?,
                );
                let (t0, t1) = (
                    self.num(id, "to_min", depth).ok_or(Fail::Bad)?,
                    self.num(id, "to_max", depth).ok_or(Fail::Bad)?,
                );
                if (f1 - f0).abs() < 1e-12 {
                    return Err(Fail::Bad);
                }
                let op = self.text(id, "operation");
                Ok(a.into_iter()
                    .enumerate()
                    .map(|(i, (name, x))| {
                        let x = match &b {
                            Some(b) => {
                                let y = b.get(i).or_else(|| b.first()).map_or(0.0, |c| c.1);
                                match op.as_str() {
                                    "subtract" => x - y,
                                    "multiply" => x * y,
                                    "divide" => x / y,
                                    "maximum" => x.max(y),
                                    "minimum" => x.min(y),
                                    _ => x + y,
                                }
                            }
                            None => x,
                        };
                        (name, t0 + (x - f0) * (t1 - t0) / (f1 - f0))
                    })
                    .collect())
            }
            _ => Err(Fail::Unsupported),
        }
    }

    fn table(&self, id: NodeId) -> Option<Dat> {
        let n = self.net.node(id)?;
        if n.type_id != "dat.table" {
            return None;
        }
        let delim = self.text(id, "delimiter");
        let delim = if delim.is_empty() {
            "tab".into()
        } else {
            delim
        };
        let t: Table = lsr_ops_dat::delim::parse(&self.text(id, "rows"), &delim);
        Some(Dat::from(t))
    }
}

struct ExprEnv<'a, 'b> {
    ev: &'a Eval<'b>,
    id: NodeId,
    refs: Vec<Option<NodeId>>,
    depth: u32,
}

impl ExprEnv<'_, '_> {
    fn node(&self, i: usize) -> Result<NodeId, EvalError> {
        self.refs
            .get(i)
            .copied()
            .flatten()
            .ok_or(EvalError::NoNode(i))
    }
}

impl Env for ExprEnv<'_, '_> {
    fn time(&self) -> f64 {
        self.ev.t
    }

    fn frame(&self) -> f64 {
        (self.ev.t * 60.0).floor()
    }

    fn me_digits(&self) -> Option<f64> {
        let name = &self.ev.net.node(self.id)?.name;
        lsr_core::expr::name_digits(name)
    }

    fn param(&self, node: usize, param: &str, index: Option<usize>) -> Result<f64, EvalError> {
        self.ev
            .drive(self.node(node)?, param, index.unwrap_or(0), self.depth + 1)
            .map_err(|_| EvalError::NoParam(node))
    }

    fn channel(&self, node: usize, channel: &str) -> Result<f64, EvalError> {
        self.ev
            .channels(self.node(node)?, self.depth + 1)
            .ok()
            .and_then(|c| c.into_iter().find(|(n, _)| n == channel))
            .map(|(_, v)| v)
            .ok_or(EvalError::NoChannel(node))
    }

    fn cell(
        &self,
        node: usize,
        slot: usize,
        row: CellAt<'_>,
        col: CellAt<'_>,
    ) -> Result<f64, EvalError> {
        let id = self.node(node)?;
        let dat = self.ev.table(id).ok_or(EvalError::NotDat(node))?;
        let v = cell_number(&dat, slot, row, col)?;
        Ok(if self.ev.perturb { v * 3.0 + 0.5 } else { v })
    }
}

/// The wave of an LFO at phase `x` (cycles).
fn wave(kind: &str, x: f64) -> f64 {
    let x = x.rem_euclid(1.0);
    match kind {
        "triangle" => 1.0 - 4.0 * ((x + 0.25).rem_euclid(1.0) - 0.5).abs(),
        "square" => {
            if x < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        "ramp" => 2.0 * x - 1.0,
        _ => (x * std::f64::consts::TAU).sin(),
    }
}

/// The parts of a Python tuple `(a, b)` split at its top-level commas; empty
/// when `source` is not one.
fn tuple_parts(source: &str) -> Vec<&str> {
    let s = source.trim();
    let Some(inner) = s.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
        return Vec::new();
    };
    let (mut depth, mut start) = (0i32, 0);
    let mut parts = Vec::new();
    for (i, c) in inner.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(inner[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(inner[start..].trim());
    // A parenthesised single expression is not a tuple; one with a call
    // (`(f(a, b))`) has its commas inside, so it stays whole.
    if parts.len() < 2 { Vec::new() } else { parts }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tuple_splits_at_top_level_commas_only() {
        assert_eq!(
            tuple_parts("(0.5 + 0.3*cos(time), 0.5 + 0.3*sin(time))"),
            ["0.5 + 0.3*cos(time)", "0.5 + 0.3*sin(time)"]
        );
        assert_eq!(tuple_parts("(max(1, 2))"), Vec::<&str>::new());
        assert_eq!(tuple_parts("time * 2"), Vec::<&str>::new());
    }

    #[test]
    fn waves_follow_the_lfo_operator() {
        assert!((wave("sine", 0.25) - 1.0).abs() < 1e-9);
        assert!(
            (wave("triangle", 0.75) + 1.0).abs() < 1e-9
                && (wave("triangle", 0.25) - 1.0).abs() < 1e-9
        );
        assert_eq!(wave("square", 0.75), -1.0);
        assert!((wave("ramp", 0.5)).abs() < 1e-9);
    }
}
