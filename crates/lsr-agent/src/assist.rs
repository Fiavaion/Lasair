//! Assist levels for the assistant (PRD A72): how much help a model gets,
//! so local models can be compared with and without it.
//!
//! - **bare**: the plain tool loop. A few rules, no operator list, no
//!   retrieval, no planned mode, no repair prompts. The model finds
//!   operators with `list_ops` and `get_schema`.
//! - **standard**: what the app does: the operator list, lexical retrieval
//!   of operator docs, the planned mode and its repair rounds.
//! - **full**: standard plus semantic retrieval of operator docs and of
//!   worked examples (embeddings from Ollama, cached), a harness that
//!   checks a plan before staging it, more repair rounds and one
//!   self-review.
//!
//! Everything here runs on the assistant's worker thread, never the render
//! thread (CLAUDE.md rule 1); embedding requests go through the egress
//! gate like every other request.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use lsr_core::Registry;
use lsr_core::graph::Network;
use lsr_core::registry::{OperatorSchema, ParamKind, ParamSchema, WireType};

use crate::assistant::{Ollama, post_json};

/// How much help the assistant gives a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Assist {
    Bare,
    #[default]
    Standard,
    Full,
}

impl Assist {
    pub fn as_str(self) -> &'static str {
        match self {
            Assist::Bare => "bare",
            Assist::Standard => "standard",
            Assist::Full => "full",
        }
    }
}

impl fmt::Display for Assist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Assist {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "bare" => Ok(Assist::Bare),
            "standard" => Ok(Assist::Standard),
            "full" => Ok(Assist::Full),
            other => Err(format!(
                "unknown assist level {other:?} (bare, standard, full)"
            )),
        }
    }
}

/// The embedding model `full` uses unless told otherwise.
pub const DEFAULT_EMBED_MODEL: &str = "qwen3-embedding:0.6b";

/// Plan rounds `full` allows (the first plan and up to six repairs).
pub const FULL_ROUNDS: u32 = 7;
/// Plan rounds the answer to a self-review may take.
pub const REVIEW_ROUNDS: u32 = 3;
/// The most model turns a request can take at `level` with `max_steps`:
/// `full` may add the self-review's rounds to its plan rounds, so it can
/// exceed `max_steps`.
pub fn effective_max_turns(level: Assist, max_steps: u32) -> u32 {
    match level {
        Assist::Full => max_steps.max(max_steps.min(FULL_ROUNDS) + REVIEW_ROUNDS),
        _ => max_steps,
    }
}

/// Operators the semantic search adds, and worked examples it injects.
const TOP_OPS: usize = 8;
const TOP_EXAMPLES: usize = 2;
/// Examples bigger than this are not indexed: they would not fit a prompt
/// (and the plan schema allows 24 nodes).
const MAX_EXAMPLE_NODES: usize = 24;

/// The rules `bare` gives: what a patch is and how to use the tools.
pub(crate) const BARE_RULES: &str = "You build node networks (patches) in Lasair, a real-time visual instrument, by calling tools. \
A patch is a graph of operator nodes, each with a type such as `img.blur`, parameters, and typed input and output sockets, joined by wires. \
Find operators with list_ops and get_schema, build with the tools, then call accept_changes, read the errors it returns, fix them and accept again until there are none. \
Do not explain; act with tools.";

pub(crate) const REVIEW_RULES: &str = "You review a node network (patch) in Lasair against the request it was built for. \
List only concrete things that are missing or wrong with respect to what the request asks: a node it names or implies that is absent, a wrong or missing wire, a parameter that contradicts the request, something that should move but does not. \
Do not suggest extras the request does not ask for. If the network satisfies the request, answer complete=true with no problems.";

// ---------------------------------------------------------------------
// Embeddings
// ---------------------------------------------------------------------

/// Something that turns text into vectors.
pub trait Embedder {
    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
    /// Names the model, for the cache key.
    fn model(&self) -> String;
}

/// Ollama's `/api/embed`.
#[derive(Debug, Clone)]
pub struct OllamaEmbedder {
    pub base: String,
    pub model: String,
}

impl OllamaEmbedder {
    pub fn new(model: &str) -> Self {
        Self {
            base: Ollama::new(model).base,
            model: model.to_owned(),
        }
    }
}

impl Embedder for OllamaEmbedder {
    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let mut out = Vec::with_capacity(texts.len());
        for batch in texts.chunks(16) {
            let body = json!({"model": self.model, "input": batch});
            let r = post_json(&format!("{}/api/embed", self.base), &[], &body)?;
            if let Some(e) = r.get("error") {
                return Err(format!("Ollama embed: {e}"));
            }
            let rows = r["embeddings"]
                .as_array()
                .ok_or("Ollama embed: no `embeddings` in the answer")?;
            if rows.len() != batch.len() {
                return Err(format!(
                    "Ollama embed: {} vectors for {} texts",
                    rows.len(),
                    batch.len()
                ));
            }
            for row in rows {
                let v: Vec<f32> = row
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|x| x.as_f64().map(|x| x as f32))
                    .collect();
                if v.is_empty() {
                    return Err("Ollama embed: an empty vector".into());
                }
                out.push(v);
            }
        }
        Ok(out)
    }

    fn model(&self) -> String {
        self.model.clone()
    }
}

fn normalise(mut v: Vec<f32>) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in &mut v {
            *x /= n;
        }
    }
    v
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// FNV-1a, as 16 hex digits: the cache key over the corpus and model.
fn fnv(parts: &[&str]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for b in p.bytes().chain([0u8]) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

// ---------------------------------------------------------------------
// The corpus: operators and worked examples
// ---------------------------------------------------------------------

fn range_text(k: &ParamKind) -> String {
    let n = |v: Option<f64>| v.map_or(String::new(), |v| v.to_string());
    match k {
        ParamKind::Float { min, max } => format!("{}..{}", n(*min), n(*max)),
        ParamKind::Int { min, max } => {
            format!("{}..{}", n(min.map(|v| v as f64)), n(max.map(|v| v as f64)))
        }
        _ => String::new(),
    }
}

fn kind_text(k: &ParamKind) -> String {
    match k {
        ParamKind::Float { .. } => format!("float[{}]", range_text(k)),
        ParamKind::Int { .. } => format!("int[{}]", range_text(k)),
        ParamKind::Menu { options } => format!("menu({})", options.join("|")),
        ParamKind::Vec { n } => format!("vec{n}"),
        ParamKind::String => "string".into(),
        ParamKind::Toggle => "toggle".into(),
        ParamKind::Pulse => "pulse".into(),
        ParamKind::Colour => "colour".into(),
        ParamKind::Path => "path".into(),
    }
}

/// One chunk per operator: the corpus of the generated operator docs
/// (`lasair docs`): type, summary, parameters with kinds, ranges and
/// defaults, inputs and outputs.
pub fn op_chunk(op: &OperatorSchema) -> String {
    let sockets = |s: &[lsr_core::registry::SocketSchema]| {
        s.iter()
            .map(|s| {
                format!(
                    "{}:{:?}{}",
                    s.name,
                    s.wire,
                    if s.optional { "?" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut t = format!(
        "{} ({}, {}). {}\nInputs: {}\nOutputs: {}\nParameters:",
        op.type_id,
        op.label,
        op.family,
        op.summary.trim(),
        sockets(&op.inputs),
        sockets(&op.outputs)
    );
    for p in &op.params {
        let _ = write!(t, "\n{} {} = {}", p.name, kind_text(&p.kind), p.default);
        if !p.help.is_empty() {
            let _ = write!(t, " - {}", p.help.trim());
        }
    }
    // Long chunks are cut: an embedding model reads a prefix well enough.
    let end = t.floor_char_boundary(3000);
    t.truncate(end);
    t
}

/// A shipped example project, as a worked example for the model.
#[derive(Debug, Clone)]
pub struct Example {
    pub id: String,
    pub description: String,
    pub network: Network,
}

impl Example {
    fn chunk(&self) -> String {
        let types: BTreeSet<&str> = self.network.nodes().map(|n| n.type_id.as_str()).collect();
        format!(
            "{}. {}\nOperators: {}",
            self.id,
            self.description,
            types.into_iter().collect::<Vec<_>>().join(", ")
        )
    }
}

/// Eval prompts, seeds and reference solutions live here. They are never
/// indexed: a solution in the prompt would leak the answer.
pub fn is_eval_path(path: &Path) -> bool {
    let p = path.to_string_lossy().replace('\\', "/");
    p.contains("milestones/m5/hard")
}

/// The `*.lasair` projects in `dir` whose main network is small and uses
/// only operators `registry` knows (Pack instances are left out: the model
/// is not offered those types). Anything under the eval directory is
/// skipped.
pub fn load_examples(dir: &Path, registry: &Registry) -> Vec<Example> {
    if is_eval_path(dir) {
        return Vec::new();
    }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "lasair") && p.is_dir())
        .filter(|p| !is_eval_path(p))
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    for d in dirs {
        let Ok(project) = lsr_format::load(&d) else {
            continue;
        };
        let Some(net) = project.main() else { continue };
        if net.is_empty()
            || net.len() > MAX_EXAMPLE_NODES
            || net.nodes().any(|n| registry.get(&n.type_id).is_none())
        {
            continue;
        }
        let id = d
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let description = project
            .description
            .split("\n\n")
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        out.push(Example {
            id,
            description,
            network: net.clone(),
        });
    }
    out
}

// ---------------------------------------------------------------------
// Retrieval
// ---------------------------------------------------------------------

/// What the semantic search found for one request.
#[derive(Debug, Clone, Default)]
pub struct Hits {
    /// Operator type IDs, best first.
    pub ops: Vec<String>,
    /// The nearest worked examples, best first.
    pub examples: Vec<Example>,
}

#[derive(Serialize, Deserialize)]
struct Cached {
    model: String,
    hash: String,
    ids: Vec<String>,
    vectors: Vec<Vec<f32>>,
}

/// Cosine search over the operator docs and the worked examples.
pub struct Retriever {
    embedder: Box<dyn Embedder>,
    ops: Vec<(String, Vec<f32>)>,
    examples: Vec<(Example, Vec<f32>)>,
    /// A query failed: say so once, then stay lexical.
    failed: bool,
    /// Why (the embedder's error), once it has failed.
    failure: Option<String>,
}

impl Retriever {
    /// Indexes the operators of `registry` and `examples` (Pack types
    /// excluded). A cache in `cache_dir` whose key (model, corpus) matches
    /// is used as it is; otherwise the corpus is embedded and the cache
    /// rewritten. Blocks on the embedder.
    pub fn build(
        registry: &Registry,
        examples: Vec<Example>,
        mut embedder: Box<dyn Embedder>,
        cache_dir: Option<&Path>,
    ) -> Result<Self, String> {
        let ops: Vec<&OperatorSchema> = registry
            .iter()
            .filter(|o| !o.type_id.starts_with(lsr_core::pack::TYPE_PREFIX))
            .collect();
        let mut ids: Vec<String> = ops.iter().map(|o| format!("op:{}", o.type_id)).collect();
        let mut texts: Vec<String> = ops.iter().map(|o| op_chunk(o)).collect();
        for e in &examples {
            ids.push(format!("ex:{}", e.id));
            texts.push(e.chunk());
        }
        let model = embedder.model();
        let mut key: Vec<&str> = vec![&model];
        key.extend(ids.iter().map(String::as_str));
        key.extend(texts.iter().map(String::as_str));
        let hash = fnv(&key);
        let file = cache_dir.map(|d| d.join("index.json"));
        let cached = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|t| serde_json::from_str::<Cached>(&t).ok())
            .filter(|c| c.hash == hash && c.model == model && c.vectors.len() == ids.len());
        let vectors = match cached {
            Some(c) => c.vectors,
            None => {
                let v: Vec<Vec<f32>> = embedder.embed(&texts)?.into_iter().map(normalise).collect();
                if v.len() != ids.len() {
                    return Err("the embedder returned the wrong number of vectors".into());
                }
                if let Some(f) = &file {
                    store(
                        f,
                        &Cached {
                            model: model.clone(),
                            hash,
                            ids: ids.clone(),
                            vectors: v.clone(),
                        },
                    );
                }
                v
            }
        };
        let mut vectors = vectors.into_iter();
        let op_index = ops
            .iter()
            .map(|o| o.type_id.clone())
            .zip(vectors.by_ref())
            .collect();
        let example_index = examples.into_iter().zip(vectors).collect();
        Ok(Self {
            embedder,
            ops: op_index,
            examples: example_index,
            failed: false,
            failure: None,
        })
    }

    /// The nearest operators and examples to `request`. `None` once the
    /// embedder has failed (it says so on stderr, once): the caller falls
    /// back to lexical retrieval.
    pub fn search(&mut self, request: &str) -> Option<Hits> {
        if self.failed {
            return None;
        }
        let query = format!(
            "Instruct: Given a request to build a visual node network, retrieve the operators and example projects it needs\nQuery: {request}"
        );
        let q = match self.embedder.embed(&[query]) {
            Ok(mut v) if v.len() == 1 => normalise(v.remove(0)),
            other => {
                self.failed = true;
                let why = other.err().unwrap_or_else(|| "no vector".into());
                self.failure = Some(why.clone());
                eprintln!(
                    "assist full: the embedding model is unavailable ({why}); using lexical retrieval"
                );
                return None;
            }
        };
        let mut ops: Vec<(f32, &String)> =
            self.ops.iter().map(|(id, v)| (dot(&q, v), id)).collect();
        ops.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(b.1)));
        let mut ex: Vec<(f32, &Example)> =
            self.examples.iter().map(|(e, v)| (dot(&q, v), e)).collect();
        ex.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
        Some(Hits {
            ops: ops.into_iter().take(TOP_OPS).map(|o| o.1.clone()).collect(),
            examples: ex
                .into_iter()
                .take(TOP_EXAMPLES)
                .map(|e| e.1.clone())
                .collect(),
        })
    }
}

/// Writes the cache atomically; a failure only costs the next run a
/// re-embed. Nothing is written into a folder removed this session.
fn store(file: &Path, c: &Cached) {
    if lsr_core::user_data::was_removed(file) {
        return;
    }
    let Some(dir) = file.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(text) = serde_json::to_string(c) else {
        return;
    };
    let tmp = file.with_extension("json.tmp");
    if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, file).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

impl fmt::Debug for Retriever {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Retriever({} operators, {} examples)",
            self.ops.len(),
            self.examples.len()
        )
    }
}

/// The help a request gets: the level, and for `full` the retriever.
#[derive(Debug)]
pub struct Assistance {
    pub level: Assist,
    pub retriever: Option<Retriever>,
    /// Why retrieval is lexical although `full` was asked for (the index
    /// could not be built).
    pub unavailable: Option<String>,
    /// The last [`Self::search`] fell back to lexical retrieval.
    last_lexical: bool,
}

impl Assistance {
    pub fn new(level: Assist) -> Self {
        Self {
            level,
            retriever: None,
            unavailable: None,
            last_lexical: false,
        }
    }

    /// `full` without semantic retrieval: `reason` (the embedder's error).
    pub fn without_retriever(mut self, reason: String) -> Self {
        self.unavailable = Some(reason);
        self
    }

    /// Why retrieval is lexical now, or `None` while it is semantic (or
    /// the level does not retrieve).
    pub fn lexical_reason(&self) -> Option<String> {
        if self.level != Assist::Full {
            return None;
        }
        match &self.retriever {
            Some(r) => r
                .failure
                .clone()
                .map(|e| format!("embeddings unavailable: {e}")),
            None => Some(format!(
                "embeddings unavailable: {}",
                self.unavailable.as_deref().unwrap_or("no index")
            )),
        }
    }

    /// The last search fell back to lexical retrieval (at `full`).
    pub fn last_search_was_lexical(&self) -> bool {
        self.last_lexical
    }

    pub fn standard() -> Self {
        Self::new(Assist::Standard)
    }

    pub fn with_retriever(mut self, retriever: Retriever) -> Self {
        self.retriever = Some(retriever);
        self
    }

    /// Semantic hits for `request`, at level `full` with a working retriever.
    pub fn search(&mut self, request: &str) -> Option<Hits> {
        if self.level != Assist::Full {
            return None;
        }
        let hits = self.retriever.as_mut().and_then(|r| r.search(request));
        self.last_lexical = hits.is_none();
        hits
    }
}

// ---------------------------------------------------------------------
// Networks as text
// ---------------------------------------------------------------------

/// `net` in the plan format the model answers with: nodes with their
/// non-default parameters (an expression stands for its value) and wires
/// `node.output` to `node.input`; a bound parameter is a wire from the SIG
/// node to the parameter.
pub fn network_json(net: &Network) -> Value {
    let nodes: Vec<Value> = net
        .nodes()
        .map(|n| {
            let params: Vec<Value> = n
                .params
                .iter()
                .filter(|(_, p)| p.bind.is_none())
                .map(|(k, p)| {
                    json!({"name": k, "value": p.expr.as_ref().map_or_else(|| p.value.clone(), |e| json!(e))})
                })
                .collect();
            json!({"name": n.name, "type": n.type_id, "params": params})
        })
        .collect();
    let name = |id| net.node(id).map_or("?", |n| n.name.as_str());
    let mut wires: Vec<Value> = net
        .wires()
        .map(|w| {
            json!({
                "from": format!("{}.{}", name(w.from.node), w.from.socket),
                "to": format!("{}.{}", name(w.to.node), w.to.socket),
            })
        })
        .collect();
    for n in net.nodes() {
        for (k, p) in &n.params {
            if let Some(b) = &p.bind {
                let channel = if b.channel.is_empty() {
                    "out"
                } else {
                    &b.channel
                };
                wires.push(json!({
                    "from": format!("{}.{channel}", name(b.node)),
                    "to": format!("{}.{k}", n.name),
                }));
            }
        }
    }
    json!({"nodes": nodes, "wires": wires})
}

/// A plain-text description of `net` for a reviewer: one line per node
/// and one per wire.
pub fn describe_network(net: &Network) -> String {
    let plan = network_json(net);
    let mut s = String::new();
    for n in plan["nodes"].as_array().into_iter().flatten() {
        let params: Vec<String> = n["params"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| format!("{} = {}", p["name"].as_str().unwrap_or("?"), p["value"]))
            .collect();
        let _ = writeln!(
            s,
            "- {} ({}){}{}",
            n["name"].as_str().unwrap_or("?"),
            n["type"].as_str().unwrap_or("?"),
            if params.is_empty() { "" } else { ": " },
            params.join(", ")
        );
    }
    let wires: Vec<&Value> = plan["wires"].as_array().into_iter().flatten().collect();
    if wires.is_empty() {
        s.push_str("No wires.\n");
    } else {
        s.push_str("Wires:\n");
        for w in wires {
            let _ = writeln!(
                s,
                "- {} -> {}",
                w["from"].as_str().unwrap_or("?"),
                w["to"].as_str().unwrap_or("?")
            );
        }
    }
    s
}

/// The worked examples as a prompt section. `planned` writes them in the
/// plan format; otherwise in the shape `build_network` takes.
pub(crate) fn examples_block(examples: &[Example], planned: bool) -> String {
    if examples.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        "\nWorked examples of similar networks (types, non-default parameters and wires); adapt them, do not copy blindly:\n",
    );
    for (i, e) in examples.iter().enumerate() {
        let plan = network_json(&e.network);
        let shown = if planned {
            plan
        } else {
            crate::assistant::plan_to_network(&plan)
        };
        let _ = writeln!(s, "Example {} - {}\n{shown}", i + 1, e.description);
    }
    s
}

// ---------------------------------------------------------------------
// Plan validation (the harness of `full`)
// ---------------------------------------------------------------------

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push(
                (prev[j] + usize::from(ca != *cb))
                    .min(prev[j + 1] + 1)
                    .min(cur[j] + 1),
            );
        }
        prev = cur;
    }
    prev[b.len()]
}

fn closest(reg: &Registry, wanted: &str, n: usize) -> Vec<String> {
    let mut all: Vec<(usize, &str)> = reg
        .iter()
        .filter(|o| !o.type_id.starts_with(lsr_core::pack::TYPE_PREFIX))
        .map(|o| (edit_distance(wanted, &o.type_id), o.type_id.as_str()))
        .collect();
    all.sort();
    all.into_iter().take(n).map(|(_, t)| t.to_owned()).collect()
}

fn names(list: impl Iterator<Item = String>, cap: usize) -> String {
    let v: Vec<String> = list.collect();
    if v.len() > cap {
        format!("{}, … ({} more)", v[..cap].join(", "), v.len() - cap)
    } else {
        v.join(", ")
    }
}

fn param_fault(node: &str, ty: &str, p: &ParamSchema, v: &Value) -> Option<String> {
    // A text value for a number, toggle, vector or colour is an expression
    // (checked when staged); one number drives every component of a vector.
    if v.is_string() && p.kind.can_follow() {
        return None;
    }
    if v.is_number() && matches!(p.kind, ParamKind::Vec { .. } | ParamKind::Colour) {
        return None;
    }
    p.validate(v).err()?;
    let what = format!("node `{node}` ({ty}): parameter `{}` = {v}", p.name);
    Some(match &p.kind {
        ParamKind::Int { .. } if v.is_number() && v.as_i64().is_none() => {
            format!("{what} must be a whole number")
        }
        ParamKind::Float { .. } | ParamKind::Int { .. } if v.is_number() => {
            format!(
                "{what} is outside the allowed range {}",
                range_text(&p.kind)
            )
        }
        ParamKind::Float { .. } | ParamKind::Int { .. } => {
            format!("{what} must be a number (or an expression string)")
        }
        ParamKind::Menu { options } => format!("{what} is not one of: {}", options.join(", ")),
        ParamKind::Toggle | ParamKind::Pulse => format!("{what} must be true or false"),
        ParamKind::Vec { n } => format!("{what} must be a list of {n} numbers"),
        ParamKind::Colour => format!("{what} must be a list of 4 numbers (r, g, b, a)"),
        ParamKind::String | ParamKind::Path => format!("{what} must be text"),
    })
}

fn split_end(s: &str, default: &str) -> (String, String) {
    match s.rsplit_once('.') {
        Some((n, k)) if !n.is_empty() && !k.is_empty() => (n.to_owned(), k.to_owned()),
        _ => (s.to_owned(), default.to_owned()),
    }
}

/// Faults in a plan that staging would reject or that leave a node
/// unwired, each naming the node and the valid options: unknown types,
/// unknown parameter names, values outside the schema, wires to nothing,
/// wire type mismatches and required inputs with no wire. `base` is the
/// network the plan is staged on (its nodes and wires count).
pub fn validate_plan(plan: &Value, base: &Network, reg: &Registry) -> Vec<String> {
    let mut out = Vec::new();
    let mut types: BTreeMap<String, String> = base
        .nodes()
        .map(|n| (n.name.clone(), n.type_id.clone()))
        .collect();
    let mut in_plan = BTreeSet::new();
    for n in plan["nodes"].as_array().into_iter().flatten() {
        let (Some(name), Some(ty)) = (n["name"].as_str(), n["type"].as_str()) else {
            out.push(format!("node {n}: needs a `name` and a `type`"));
            continue;
        };
        if !in_plan.insert(name.to_owned()) {
            out.push(format!(
                "node name `{name}` is used twice; every node needs its own name"
            ));
            continue;
        }
        let Some(op) = reg.get(ty) else {
            out.push(format!(
                "node `{name}`: unknown operator type `{ty}`; the closest are: {}",
                closest(reg, ty, 5).join(", ")
            ));
            continue;
        };
        if let Some(old) = base.node_by_name(name).filter(|o| o.type_id != ty) {
            out.push(format!(
                "node `{name}` already exists as `{}`; keep that type, or use another name",
                old.type_id
            ));
        }
        types.insert(name.to_owned(), ty.to_owned());
        let listed: Vec<(String, Value)> = match &n["params"] {
            Value::Array(l) => l
                .iter()
                .filter_map(|p| Some((p["name"].as_str()?.to_owned(), p["value"].clone())))
                .collect(),
            Value::Object(o) => o.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => Vec::new(),
        };
        for (pname, value) in listed {
            match op.params.iter().find(|p| p.name == pname) {
                None => out.push(format!(
                    "node `{name}` ({ty}): no parameter `{pname}`; its parameters are: {}",
                    names(op.params.iter().map(|p| p.name.clone()), 24)
                )),
                Some(p) => out.extend(param_fault(name, ty, p, &value)),
            }
        }
    }
    let mut wired: BTreeSet<(String, String)> = base
        .wires()
        .filter_map(|w| Some((base.node(w.to.node)?.name.clone(), w.to.socket)))
        .collect();
    let mut sources: BTreeMap<(String, String), String> = BTreeMap::new();
    for w in plan["wires"].as_array().into_iter().flatten() {
        let (from, to) = (
            w["from"].as_str().unwrap_or_default(),
            w["to"].as_str().unwrap_or_default(),
        );
        let (src, out_name) = split_end(from, "out");
        let (dst, in_name) = split_end(to, "in");
        let label = format!("wire {src}.{out_name} -> {dst}.{in_name}");
        let mut known = true;
        for (node, role) in [(&src, "from"), (&dst, "to")] {
            if !types.contains_key(node) {
                known = false;
                out.push(format!(
                    "{label}: `{role}` names no node `{node}`; the nodes are: {}",
                    names(types.keys().cloned(), 30)
                ));
            }
        }
        if !known {
            continue;
        }
        let (Some(sop), Some(dop)) = (reg.get(&types[&src]), reg.get(&types[&dst])) else {
            continue;
        };
        let dst_input = dop.inputs.iter().find(|i| i.name == in_name);
        let src_output = sop.outputs.iter().find(|o| o.name == out_name);
        let src_is_sig = sop.outputs.iter().any(|o| o.wire == WireType::Sig);
        let dst_param = dop.params.iter().find(|p| p.name == in_name);
        match (dst_input, dst_param) {
            (Some(input), _) => {
                let Some(output) = src_output else {
                    out.push(format!(
                        "{label}: node `{src}` ({}) has no output `{out_name}`; its outputs are: {}",
                        sop.type_id,
                        names(
                            sop.outputs.iter().map(|o| format!("{} ({:?})", o.name, o.wire)),
                            12
                        )
                    ));
                    continue;
                };
                if output.wire != input.wire {
                    out.push(format!(
                        "{label}: types differ; `{src}.{out_name}` carries {:?} but `{dst}.{in_name}` takes {:?}",
                        output.wire, input.wire
                    ));
                    continue;
                }
                if let Some(prev) = sources.insert((dst.clone(), in_name.clone()), src.clone())
                    && prev != src
                {
                    out.push(format!(
                        "input `{dst}.{in_name}` is wired from both `{prev}` and `{src}`; an input takes one wire"
                    ));
                }
                wired.insert((dst, in_name));
            }
            (None, Some(p)) if src_is_sig => {
                if !p.kind.can_follow() {
                    out.push(format!(
                        "{label}: parameter `{in_name}` of `{dst}` ({}) cannot follow a channel",
                        dop.type_id
                    ));
                }
            }
            (None, Some(_)) => out.push(format!(
                "{label}: `{in_name}` is a parameter of `{dst}`; only a SIG node can be wired to a parameter, and `{src}` ({}) is not one",
                sop.type_id
            )),
            (None, None) => out.push(format!(
                "{label}: node `{dst}` ({}) has no input `{in_name}`; its inputs are: {}",
                dop.type_id,
                names(
                    dop.inputs.iter().map(|i| format!("{} ({:?})", i.name, i.wire)),
                    12
                )
            )),
        }
    }
    for name in &in_plan {
        let Some(op) = types.get(name).and_then(|t| reg.get(t)) else {
            continue;
        };
        for input in op.inputs.iter().filter(|i| !i.optional) {
            if wired.contains(&(name.clone(), input.name.clone())) {
                continue;
            }
            let candidates: Vec<String> = in_plan
                .iter()
                .filter(|s| *s != name)
                .filter_map(|s| {
                    let sop = types.get(s).and_then(|t| reg.get(t))?;
                    let o = sop.outputs.iter().find(|o| o.wire == input.wire)?;
                    Some(format!("{s}.{}", o.name))
                })
                .collect();
            out.push(format!(
                "node `{name}` ({}): required input `{}` ({:?}) has no wire; wire a {:?} output into `{name}.{}`{}",
                op.type_id,
                input.name,
                input.wire,
                input.wire,
                input.name,
                if candidates.is_empty() {
                    String::new()
                } else {
                    format!(" (candidates: {})", names(candidates.into_iter(), 8))
                }
            ));
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod testkit {
    use lsr_core::registry::{OpFamily, OperatorSchema, ParamSchema, SocketSchema, WireType};

    use super::*;

    fn sock(n: &str, w: WireType, optional: bool) -> SocketSchema {
        SocketSchema {
            name: n.into(),
            wire: w,
            optional,
            feedback: false,
        }
    }

    fn op(
        id: &str,
        family: OpFamily,
        label: &str,
        summary: &str,
        params: Vec<ParamSchema>,
        inputs: Vec<SocketSchema>,
        outputs: Vec<SocketSchema>,
    ) -> OperatorSchema {
        OperatorSchema {
            type_id: id.into(),
            family,
            label: label.into(),
            summary: summary.into(),
            version: 1,
            params,
            inputs,
            outputs,
            capabilities: vec![],
            cook_budget_us: 100,
            time_dependent: false,
            sink: false,
        }
    }

    /// Three operators, one with a required input: stable text for the
    /// golden prompts.
    pub(crate) fn tiny_registry() -> Registry {
        let mut reg = Registry::default();
        reg.register(op(
            "img.noise",
            OpFamily::Img,
            "Noise",
            "Animated fractal noise. Drifts with time.",
            vec![
                ParamSchema::float("scale", "Scale", 4.0, Some(0.1), Some(64.0)),
                ParamSchema::int("octaves", "Octaves", 3, Some(1), Some(8)),
            ],
            vec![],
            vec![sock("out", WireType::Img, false)],
        ))
        .unwrap();
        reg.register(op(
            "img.blur",
            OpFamily::Img,
            "Blur",
            "Gaussian blur of an image.",
            vec![
                ParamSchema::float("size", "Size", 4.0, Some(0.0), None),
                ParamSchema::menu("mode", "Mode", &["box", "gauss"], "gauss"),
            ],
            vec![sock("in", WireType::Img, false)],
            vec![sock("out", WireType::Img, false)],
        ))
        .unwrap();
        reg.register(op(
            "sig.lfo",
            OpFamily::Sig,
            "LFO",
            "A low-frequency oscillator. Channel `lfo`.",
            vec![ParamSchema::float(
                "frequency",
                "Frequency",
                1.0,
                Some(0.0),
                None,
            )],
            vec![],
            vec![sock("out", WireType::Sig, false)],
        ))
        .unwrap();
        reg
    }

    /// A bag-of-words embedder: words hashed into 128 buckets, so texts
    /// that share words are close. Counts the texts it embedded.
    pub(crate) struct WordEmbedder {
        pub(crate) name: String,
        pub(crate) embedded: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl WordEmbedder {
        pub(crate) fn new(name: &str) -> Self {
            Self {
                name: name.into(),
                embedded: Default::default(),
            }
        }
    }

    impl Embedder for WordEmbedder {
        fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
            self.embedded.set(self.embedded.get() + texts.len());
            Ok(texts
                .iter()
                .map(|t| {
                    // Like an instruction-aware model, ignore the instruction.
                    let t = t.rsplit_once("\nQuery: ").map_or(t.as_str(), |(_, q)| q);
                    let mut v = vec![0.0f32; 128];
                    for w in t
                        .to_ascii_lowercase()
                        .split(|c: char| !c.is_alphanumeric())
                        .filter(|w| w.len() > 2)
                    {
                        let h = fnv(&[w]);
                        v[usize::from_str_radix(&h[..4], 16).unwrap() % 128] += 1.0;
                    }
                    v
                })
                .collect())
        }

        fn model(&self) -> String {
            self.name.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::testkit::*;
    use super::*;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn real_registry() -> Registry {
        let mut reg = Registry::default();
        lsr_ops_img::register(&mut reg);
        lsr_ops_sig::register(&mut reg);
        lsr_ops_dat::register(&mut reg);
        reg
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lsr-assist-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let t = to.join(e.file_name());
            if e.path().is_dir() {
                copy(&e.path(), &t);
            } else {
                std::fs::copy(e.path(), t).unwrap();
            }
        }
    }

    #[test]
    fn levels_parse() {
        for l in [Assist::Bare, Assist::Standard, Assist::Full] {
            assert_eq!(l.to_string().parse::<Assist>(), Ok(l));
        }
        assert!("everything".parse::<Assist>().is_err());
        assert_eq!(Assist::default(), Assist::Standard);
    }

    #[test]
    fn semantic_retrieval_ranks_the_relevant_operator_first() {
        let reg = real_registry();
        let mut r =
            Retriever::build(&reg, Vec::new(), Box::new(WordEmbedder::new("fake")), None).unwrap();
        let hits = r.search("a kaleidoscope pattern").unwrap();
        assert_eq!(hits.ops[0], "img.kaleidoscope", "{:?}", hits.ops);
        let hits = r.search("a low frequency oscillator").unwrap();
        assert!(hits.ops.iter().any(|o| o == "sig.lfo"), "{:?}", hits.ops);
    }

    #[test]
    fn the_nearest_examples_come_back() {
        let reg = real_registry();
        let examples = load_examples(&repo().join("examples"), &reg);
        assert!(examples.len() >= 3, "{}", examples.len());
        let mut r =
            Retriever::build(&reg, examples, Box::new(WordEmbedder::new("fake")), None).unwrap();
        let hits = r.search("kaleidoscope tunnel of noise").unwrap();
        assert_eq!(hits.examples.len(), TOP_EXAMPLES);
        assert_eq!(hits.examples[0].id, "kaleidoscope-tunnel");
    }

    #[test]
    fn the_index_is_cached_by_model_and_corpus() {
        let reg = tiny_registry();
        let dir = scratch("cache");
        let embedder = WordEmbedder::new("m1");
        let count = embedder.embedded.clone();
        Retriever::build(&reg, Vec::new(), Box::new(embedder), Some(&dir)).unwrap();
        assert_eq!(count.get(), 3);
        assert!(dir.join("index.json").is_file());
        // Same model, same corpus: read from the cache, nothing embedded.
        let again = WordEmbedder::new("m1");
        let count = again.embedded.clone();
        let mut r = Retriever::build(&reg, Vec::new(), Box::new(again), Some(&dir)).unwrap();
        assert_eq!(count.get(), 0);
        assert_eq!(r.search("blur").unwrap().ops.len(), 3);
        assert_eq!(count.get(), 1, "only the query");
        // Another model: the index is rebuilt.
        let other = WordEmbedder::new("m2");
        let count = other.embedded.clone();
        Retriever::build(&reg, Vec::new(), Box::new(other), Some(&dir)).unwrap();
        assert_eq!(count.get(), 3);
        // A changed corpus: rebuilt.
        let mut bigger = tiny_registry();
        bigger
            .register(real_registry().get("img.level").unwrap().clone())
            .unwrap();
        let again = WordEmbedder::new("m2");
        let count = again.embedded.clone();
        Retriever::build(&bigger, Vec::new(), Box::new(again), Some(&dir)).unwrap();
        assert_eq!(count.get(), 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unavailable_embedder_falls_back_once() {
        struct Down;
        impl Embedder for Down {
            fn embed(&mut self, _: &[String]) -> Result<Vec<Vec<f32>>, String> {
                Err("connection refused".into())
            }
            fn model(&self) -> String {
                "down".into()
            }
        }
        assert!(Retriever::build(&tiny_registry(), Vec::new(), Box::new(Down), None).is_err());
        // Built, then the service goes away: search says None, every time.
        let mut r = Retriever::build(
            &tiny_registry(),
            Vec::new(),
            Box::new(WordEmbedder::new("fake")),
            None,
        )
        .unwrap();
        r.embedder = Box::new(Down);
        assert!(r.search("blur").is_none());
        assert!(r.failed);
        assert!(r.search("blur").is_none());
        // The assistance says so, and which search was lexical.
        let mut a = Assistance::new(Assist::Full).with_retriever(
            Retriever::build(
                &tiny_registry(),
                Vec::new(),
                Box::new(WordEmbedder::new("fake")),
                None,
            )
            .unwrap(),
        );
        assert!(a.search("blur").is_some());
        assert!(a.lexical_reason().is_none() && !a.last_search_was_lexical());
        a.retriever.as_mut().unwrap().embedder = Box::new(Down);
        assert!(a.search("blur").is_none());
        assert!(a.last_search_was_lexical());
        assert!(
            a.lexical_reason()
                .unwrap()
                .contains("embeddings unavailable")
        );
        let none = Assistance::new(Assist::Full).without_retriever("no model".into());
        assert_eq!(
            none.lexical_reason().unwrap(),
            "embeddings unavailable: no model"
        );
        assert!(Assistance::standard().lexical_reason().is_none());
    }

    #[test]
    fn full_can_take_more_turns_than_max_steps() {
        assert_eq!(effective_max_turns(Assist::Full, 40), 40);
        assert_eq!(effective_max_turns(Assist::Full, 5), 8);
        assert_eq!(effective_max_turns(Assist::Full, 1), 4);
        assert_eq!(effective_max_turns(Assist::Standard, 5), 5);
    }

    /// Eval seeds and reference solutions must never reach a prompt.
    #[test]
    fn the_examples_index_excludes_the_eval_directory() {
        let hard = repo().join("docs/milestones/m5/hard");
        // The folder really holds projects, or the test proves nothing.
        let solutions = hard.join("solutions");
        let held = std::fs::read_dir(&solutions)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "lasair"))
            .count();
        assert!(held >= 5, "{held}");
        let reg = real_registry();
        assert!(load_examples(&solutions, &reg).is_empty());
        assert!(load_examples(&hard.join("seeds"), &reg).is_empty());
        assert!(is_eval_path(&solutions.join("h-worn-tape.lasair")));
        // And nothing the shipped examples yield comes from there.
        let shipped = load_examples(&repo().join("examples"), &reg);
        assert!(!shipped.is_empty());
        assert!(
            shipped.iter().all(|e| !e.id.starts_with("h-")),
            "{shipped:?}"
        );
        // A hard-tier project copied elsewhere under the same path is
        // skipped by path too.
        let tmp = scratch("examples").join("docs/milestones/m5/hard");
        copy(
            &solutions.join("h-worn-tape.lasair"),
            &tmp.join("h-worn-tape.lasair"),
        );
        assert!(load_examples(&tmp, &reg).is_empty());
    }

    fn plan(nodes: Value, wires: Value) -> Value {
        json!({"nodes": nodes, "wires": wires})
    }

    fn faults(p: &Value) -> Vec<String> {
        validate_plan(p, &Network::default(), &tiny_registry())
    }

    #[test]
    fn a_good_plan_has_no_faults() {
        let p = plan(
            json!([
                {"name": "noise1", "type": "img.noise", "params": [{"name": "scale", "value": 8}]},
                {"name": "blur1", "type": "img.blur", "params": [{"name": "size", "value": "4 + sin(time)"}, {"name": "mode", "value": "box"}]},
                {"name": "lfo1", "type": "sig.lfo", "params": []}
            ]),
            json!([
                {"from": "noise1.out", "to": "blur1.in"},
                {"from": "lfo1.out", "to": "blur1.size"}
            ]),
        );
        assert_eq!(faults(&p), Vec::<String>::new());
    }

    #[test]
    fn validation_names_each_class_of_fault() {
        let n = |name: &str, ty: &str, params: Value| json!({"name": name, "type": ty, "params": params});
        let one = |nodes: Value, wires: Value| faults(&plan(nodes, wires)).join("\n");

        // Unknown type: the closest real ones are offered.
        let f = one(json!([n("b", "img.blurr", json!([]))]), json!([]));
        assert!(
            f.contains("`b`")
                && f.contains("unknown operator type `img.blurr`")
                && f.contains("img.blur"),
            "{f}"
        );

        // Unknown parameter: the valid names are listed.
        let f = one(
            json!([n(
                "noise1",
                "img.noise",
                json!([{"name": "speed", "value": 1}])
            )]),
            json!([]),
        );
        assert!(
            f.contains("noise1")
                && f.contains("no parameter `speed`")
                && f.contains("scale, octaves"),
            "{f}"
        );

        // Out of range (float, int), a fraction for an int, a bad menu entry.
        let f = one(
            json!([n(
                "noise1",
                "img.noise",
                json!([
                {"name": "scale", "value": 500},
                {"name": "octaves", "value": 20},
                {"name": "octaves", "value": 2.5}])
            )]),
            json!([]),
        );
        assert!(
            f.contains("`scale` = 500 is outside the allowed range 0.1..64"),
            "{f}"
        );
        assert!(
            f.contains("`octaves` = 20 is outside the allowed range 1..8"),
            "{f}"
        );
        assert!(f.contains("`octaves` = 2.5 must be a whole number"), "{f}");
        let f = one(
            json!([
                n("noise1", "img.noise", json!([])),
                n(
                    "blur1",
                    "img.blur",
                    json!([{"name": "mode", "value": "soft"}])
                )
            ]),
            json!([{"from": "noise1.out", "to": "blur1.in"}]),
        );
        assert!(
            f.contains("blur1") && f.contains("is not one of: box, gauss"),
            "{f}"
        );

        // Wires to nothing, to a missing socket, between mismatched types.
        let nodes = json!([
            n("noise1", "img.noise", json!([])),
            n("blur1", "img.blur", json!([])),
            n("lfo1", "sig.lfo", json!([]))
        ]);
        let f = one(
            nodes.clone(),
            json!([{"from": "nois1.out", "to": "blur1.in"}]),
        );
        assert!(f.contains("names no node `nois1`"), "{f}");
        let f = one(
            nodes.clone(),
            json!([{"from": "noise1.out", "to": "blur1.image"}]),
        );
        assert!(
            f.contains("blur1") && f.contains("no input `image`") && f.contains("in (Img)"),
            "{f}"
        );
        let f = one(
            nodes.clone(),
            json!([{"from": "noise1.pixels", "to": "blur1.in"}]),
        );
        assert!(
            f.contains("no output `pixels`") && f.contains("out (Img)"),
            "{f}"
        );
        let f = one(
            nodes.clone(),
            json!([{"from": "lfo1.out", "to": "blur1.in"}]),
        );
        assert!(
            f.contains("types differ") && f.contains("Sig") && f.contains("Img"),
            "{f}"
        );
        let f = one(
            nodes.clone(),
            json!([{"from": "noise1.out", "to": "blur1.size"}]),
        );
        assert!(
            f.contains("only a SIG node can be wired to a parameter"),
            "{f}"
        );

        // A required input with no wire, with the nodes that could feed it.
        let f = one(nodes, json!([]));
        assert!(
            f.contains("node `blur1` (img.blur): required input `in` (Img) has no wire")
                && f.contains("candidates: noise1.out"),
            "{f}"
        );

        // Duplicate names.
        let f = one(
            json!([
                n("a", "img.noise", json!([])),
                n("a", "img.noise", json!([]))
            ]),
            json!([]),
        );
        assert!(f.contains("`a` is used twice"), "{f}");
    }

    fn built(nodes: Value, wires: Value) -> crate::workspace::NetworkWorkspace {
        let mut ws = crate::workspace::NetworkWorkspace::new(tiny_registry());
        let mut agent = crate::Agent::new(
            crate::Scope::defaults()
                .into_iter()
                .chain([crate::Scope::Accept])
                .collect(),
            crate::ShowLock::default(),
        );
        agent.call(
            &mut ws,
            "build_network",
            &json!({"nodes": nodes, "wires": wires}),
        );
        agent.accept(&mut ws, "test").unwrap();
        ws
    }

    #[test]
    fn wires_already_in_the_project_count() {
        let ws = built(
            json!([{"name": "noise1", "type": "img.noise"}, {"name": "blur1", "type": "img.blur"}]),
            json!([{"from": "noise1", "to": "blur1", "input": "in"}]),
        );
        let reg = tiny_registry();
        // The plan keeps blur1 and says nothing of its wire: still wired.
        let p = plan(
            json!([{"name": "blur1", "type": "img.blur", "params": [{"name": "size", "value": 9}]}]),
            json!([]),
        );
        assert_eq!(validate_plan(&p, &ws.network, &reg), Vec::<String>::new());
        // Re-typing a kept node is a fault.
        let p = plan(
            json!([{"name": "blur1", "type": "img.noise", "params": []}]),
            json!([]),
        );
        let f = validate_plan(&p, &ws.network, &reg).join("\n");
        assert!(f.contains("`blur1` already exists as `img.blur`"), "{f}");
    }

    #[test]
    fn a_network_reads_back_as_a_plan() {
        let ws = built(
            json!([
                {"name": "noise1", "type": "img.noise", "params": {"scale": "2 + sin(time)"}},
                {"name": "lfo1", "type": "sig.lfo"},
                {"name": "blur1", "type": "img.blur", "params": {"mode": "box"}}]),
            json!([
                {"from": "noise1", "to": "blur1", "input": "in"},
                {"from": "lfo1", "to": "blur1", "input": "size"}]),
        );
        let j = network_json(&ws.network);
        let text = j.to_string();
        assert!(text.contains("2 + sin(time)"), "{text}");
        assert!(
            text.contains(r#"{"from":"noise1.out","to":"blur1.in"}"#),
            "{text}"
        );
        assert!(
            text.contains(r#"{"from":"lfo1.out","to":"blur1.size"}"#),
            "bound parameter as a wire: {text}"
        );
        // What it prints validates against the registry it came from.
        assert_eq!(
            validate_plan(&j, &Network::default(), &tiny_registry()),
            Vec::<String>::new()
        );
        let d = describe_network(&ws.network);
        assert!(
            d.contains("- noise1 (img.noise): scale = \"2 + sin(time)\""),
            "{d}"
        );
        assert!(d.contains("- noise1.out -> blur1.in"), "{d}");
    }
}
