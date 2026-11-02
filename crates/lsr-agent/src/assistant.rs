//! The assistant (FR-G4): prompt to network with a language model over the
//! typed tools — generate, stage, accept (or hand to a person), read the
//! errors, repair — grounded in the operator catalogue the registry
//! generates (the same corpus as the operator docs).
//!
//! Models: Ollama on this machine by default (A8); Anthropic only when a
//! project opts in, and then through the egress gate, so Sovereign mode
//! refuses it before any packet leaves.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use lsr_ai::models::DownloadOptions;
use lsr_core::registry::ParamKind;

use crate::assist::{
    Assist, Assistance, BARE_RULES, FULL_ROUNDS, Hits, REVIEW_ROUNDS, REVIEW_RULES,
    describe_network, examples_block, network_json, validate_plan,
};
use crate::tools::{Agent, Output, output_text};
use crate::workspace::Workspace;

/// A chat model that can call tools.
pub trait Llm {
    /// The next assistant message (`{"content": …, "tool_calls": [{"name", "arguments"}]}`)
    /// for `messages` (OpenAI-style roles: system, user, assistant, tool).
    fn chat(&mut self, messages: &[Value], tools: &[Value]) -> Result<Value, String>;
    fn name(&self) -> String;
    /// A reply constrained to the JSON `schema`, for models that support
    /// structured output (`None`: not supported; [`build`] then uses tools).
    fn json(&mut self, _messages: &[Value], _schema: &Value) -> Option<Result<Value, String>> {
        None
    }
}

/// Ollama's `/api/chat` with tools.
#[derive(Debug, Clone)]
pub struct Ollama {
    pub base: String,
    pub model: String,
}

impl Ollama {
    pub fn new(model: &str) -> Self {
        Self {
            base: std::env::var("OLLAMA_HOST")
                .map(|h| {
                    if h.starts_with("http") {
                        h
                    } else {
                        format!("http://{h}")
                    }
                })
                .unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
            model: model.to_owned(),
        }
    }
}

pub(crate) fn post_json(
    url: &str,
    headers: &[(&str, &str)],
    body: &Value,
) -> Result<Value, String> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let hostport = rest.split('/').next().unwrap_or_default();
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h, p.parse().unwrap_or(80)),
        None => (hostport, if url.starts_with("https") { 443 } else { 80 }),
    };
    // The egress gate decides (Sovereign mode refuses the internet).
    lsr_ai::egress::resolve(host, port).map_err(|e| e.to_string())?;
    // The gated agent carries the TLS provider and ignores proxies from the
    // environment, which would bypass the gate.
    let refused = Arc::new(Mutex::new(None));
    let agent = lsr_ai::models::gated_agent(&DownloadOptions::default(), refused.clone());
    let mut req = agent
        .post(url)
        .config()
        .timeout_global(Some(Duration::from_secs(300)))
        .timeout_recv_response(Some(Duration::from_secs(300)))
        .build()
        .header("content-type", "application/json");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let mut resp = req.send(body.to_string()).map_err(|e| {
        match refused.lock().ok().and_then(|mut r| r.take()) {
            Some(host) => format!("{url}: Sovereign mode refused {host}"),
            None => format!("{url}: {e}"),
        }
    })?;
    let status = resp.status().as_u16();
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    if !(200..300).contains(&status) {
        return Err(format!(
            "{url} answered {status}: {}",
            text.get(..300).unwrap_or(&text)
        ));
    }
    serde_json::from_str(&text)
        .map_err(|e| format!("{url}: {e}: {}", text.get(..300).unwrap_or(&text)))
}

impl Llm for Ollama {
    fn chat(&mut self, messages: &[Value], tools: &[Value]) -> Result<Value, String> {
        let body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "stream": false,
            // Thinking models (qwen3.6, gemma4) otherwise spend the whole
            // reply budget thinking and answer with nothing.
            "think": false,
            "options": {"temperature": 0.2, "num_ctx": 32768, "num_predict": 2048},
        });
        let r = post_json(&format!("{}/api/chat", self.base), &[], &body)?;
        if let Some(e) = r.get("error") {
            return Err(format!("Ollama: {e}"));
        }
        let m = &r["message"];
        let calls: Vec<Value> = m["tool_calls"]
            .as_array()
            .into_iter()
            .flatten()
            .map(
                |c| json!({"name": c["function"]["name"], "arguments": c["function"]["arguments"]}),
            )
            .collect();
        Ok(json!({"content": m["content"], "tool_calls": calls}))
    }

    fn name(&self) -> String {
        format!("ollama:{}", self.model)
    }

    fn json(&mut self, messages: &[Value], schema: &Value) -> Option<Result<Value, String>> {
        let body = json!({
            "model": self.model,
            "messages": messages,
            "format": schema,
            "stream": false,
            "think": false,
            "options": {"temperature": 0.2, "num_ctx": 32768, "num_predict": 4096},
        });
        Some((|| {
            let r = post_json(&format!("{}/api/chat", self.base), &[], &body)?;
            if let Some(e) = r.get("error") {
                return Err(format!("Ollama: {e}"));
            }
            let text = r["message"]["content"].as_str().unwrap_or_default();
            serde_json::from_str(text).map_err(|e| {
                let tail = text.floor_char_boundary(text.len().saturating_sub(300));
                format!("Ollama: not JSON ({e}): …{}", &text[tail..])
            })
        })())
    }
}

/// Anthropic's Messages API (opt-in per project; `ANTHROPIC_API_KEY`).
#[derive(Debug, Clone)]
pub struct Anthropic {
    pub key: String,
    pub model: String,
}

impl Llm for Anthropic {
    fn chat(&mut self, messages: &[Value], tools: &[Value]) -> Result<Value, String> {
        let system = messages
            .iter()
            .filter(|m| m["role"] == "system")
            .filter_map(|m| m["content"].as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        // OpenAI-style history → Anthropic content blocks.
        let mut msgs: Vec<Value> = Vec::new();
        for m in messages.iter().filter(|m| m["role"] != "system") {
            match m["role"].as_str() {
                Some("tool") => msgs.push(json!({"role": "user", "content": [{
                    "type": "tool_result", "tool_use_id": m["tool_call_id"], "content": m["content"]}]})),
                Some("assistant") => {
                    let mut blocks = Vec::new();
                    if let Some(t) = m["content"].as_str().filter(|t| !t.is_empty()) {
                        blocks.push(json!({"type": "text", "text": t}));
                    }
                    for c in m["tool_calls"].as_array().into_iter().flatten() {
                        blocks.push(json!({"type": "tool_use", "id": c["id"], "name": c["name"], "input": c["arguments"]}));
                    }
                    msgs.push(json!({"role": "assistant", "content": blocks}));
                }
                _ => msgs.push(json!({"role": "user", "content": m["content"]})),
            }
        }
        let tools: Vec<Value> = tools
            .iter()
            .map(|t| json!({"name": t["function"]["name"], "description": t["function"]["description"], "input_schema": t["function"]["parameters"]}))
            .collect();
        let body = json!({"model": self.model, "max_tokens": 4096, "system": system, "messages": msgs, "tools": tools});
        let r = post_json(
            "https://api.anthropic.com/v1/messages",
            &[
                ("x-api-key", &self.key),
                ("anthropic-version", "2023-06-01"),
            ],
            &body,
        )?;
        if let Some(e) = r.get("error") {
            return Err(format!("Anthropic: {e}"));
        }
        let mut text = String::new();
        let mut calls = Vec::new();
        for b in r["content"].as_array().into_iter().flatten() {
            match b["type"].as_str() {
                Some("text") => text.push_str(b["text"].as_str().unwrap_or_default()),
                Some("tool_use") => {
                    calls.push(json!({"id": b["id"], "name": b["name"], "arguments": b["input"]}))
                }
                _ => {}
            }
        }
        Ok(json!({"content": text, "tool_calls": calls}))
    }

    fn name(&self) -> String {
        format!("anthropic:{}", self.model)
    }
}

/// The system prompt: how to work, and the operator catalogue.
pub fn system_prompt(ws: &dyn Workspace) -> String {
    let mut s = String::from(
        "You build node networks (patches) in Lasair, a real-time visual instrument, by calling tools. \
Rules:\n\
- Build with ONE build_network call holding every node, its parameters and every wire; use only the operator types, parameter names and input names listed (call get_schema for any other type first).\n\
- Wires connect an output to an input of the same wire type (IMG to IMG, SIG to SIG, GEO to GEO). Most image nodes take their image on input `in`; check the schema.\n\
- Changes are staged. When the network is built, call accept_changes, then read the errors it returns and fix them (set_param, connect, delete_node) and accept again, until there are no errors.\n\
- Animate with expressions such as `0.5 + 0.5 * sin(time)` in set_param, or bind a parameter to a SIG node's channel. Expressions must stay in the native subset (numbers, arithmetic, comparisons, `a if c else b`, math functions, clamp, lerp, time, frame, me.digits, op('node').par.name, op('node')['channel']): anything else is Python, refused without the python.exec scope. Vectors and colours (a transform's scale, a colour) animate the same way: the one number drives every component.\n\
- Do not explain; act with tools. Stop when the network is accepted with no errors.\n\nOperators (type — summary):\n",
    );
    // Image and channel operators first: a small model copies the prefix
    // of whatever it read first (an alphabetical list opens with `ai.`).
    let order = ["img.", "sig.", "pnt.", "msh.", "mat.", "cmp.", "dat."];
    let rank = |t: &str| {
        order
            .iter()
            .position(|p| t.starts_with(p))
            .unwrap_or(order.len())
    };
    let mut ops: Vec<_> = ws.registry().iter().collect();
    ops.sort_by_key(|op| rank(&op.type_id));
    for op in ops {
        if op.type_id.starts_with(lsr_core::pack::TYPE_PREFIX) {
            continue;
        }
        let first = op.summary.split(". ").next().unwrap_or_default();
        let _ = writeln!(s, "{} — {}", op.type_id, first.trim_end_matches('.'));
    }
    s
}

/// Tool calls a model wrote into its text instead of the tool-call field
/// (qwen2.5-coder 7B through Ollama does this): every JSON object in the
/// text, fenced or not, whose `name` is one of the offered tools.
fn calls_in_text(text: &str, tools: &[Value], is_op: impl Fn(&str) -> bool) -> Vec<Value> {
    let known = |n: &str| tools.iter().any(|t| t["function"]["name"] == n) || is_op(n);
    let mut calls = Vec::new();
    let mut i = 0;
    while let Some(off) = text[i..].find('{') {
        let start = i + off;
        let mut values = serde_json::Deserializer::from_str(&text[start..]).into_iter::<Value>();
        match values.next() {
            Some(Ok(v)) if v["name"].as_str().is_some_and(known) => {
                let args = v.get("arguments").or_else(|| v.get("parameters"));
                calls.push(
                    json!({"name": v["name"], "arguments": args.cloned().unwrap_or(json!({}))}),
                );
                i = start + values.byte_offset();
            }
            _ => i = start + 1,
        }
    }
    calls
}

/// Words of a text, lower case, with a plural or verb ending taken off
/// (`blurred` → `blur`, `gradients` → `gradient`).
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(|w| {
            let w = w.to_ascii_lowercase();
            for end in ["ing", "red", "ed", "es", "s"] {
                if let Some(stem) = w.strip_suffix(end)
                    && stem.len() > 3
                {
                    return stem.to_owned();
                }
            }
            w
        })
        .collect()
}

/// Operators every patch tends to need, always described in full.
const CORE_OPS: &[&str] = &[
    "img.noise",
    "img.ramp",
    "img.constant",
    "img.composite",
    "img.blend",
    "img.transform",
    "img.level",
    "sig.lfo",
    "sig.math",
];

/// The operators that match the request best (by the words of their type,
/// label and summary), plus [`CORE_OPS`], with their parameters and
/// sockets written compactly, so a small model needs no schema lookups.
pub fn relevant_ops(ws: &dyn Workspace, request: &str) -> String {
    ops_block(&lexical_ops(ws, request), true)
}

/// The operators [`relevant_ops`] describes, best match first.
pub(crate) fn lexical_ops<'a>(
    ws: &'a dyn Workspace,
    request: &str,
) -> Vec<&'a lsr_core::registry::OperatorSchema> {
    let asked = words(request);
    let ops: Vec<&lsr_core::registry::OperatorSchema> = ws
        .registry()
        .iter()
        .filter(|op| !op.type_id.starts_with(lsr_core::pack::TYPE_PREFIX))
        .collect();
    // Same word, or one a prefix of the other (`displac` / `displace`).
    let like = |a: &str, b: &str| {
        a == b || a.len().min(b.len()) >= 4 && (a.starts_with(b) || b.starts_with(a))
    };
    let hit = |ws: &[String], w: &str| ws.iter().any(|x| like(x, w));
    let docs: Vec<(Vec<String>, Vec<String>)> = ops
        .iter()
        .map(|op| {
            (
                words(&format!(
                    "{} {}",
                    op.type_id.replace(['.', '_'], " "),
                    op.label
                )),
                words(&op.summary),
            )
        })
        .collect();
    // A word most operators mention ("time", "colour") says little; a rare
    // one ("kaleidoscope", "hue") a lot.
    let n = ops.len() as f64;
    let weight: Vec<f64> = asked
        .iter()
        .map(|w| {
            let df = docs.iter().filter(|(a, b)| hit(a, w) || hit(b, w)).count() as f64;
            ((n + 1.0) / (df + 1.0)).ln()
        })
        .collect();
    let mut scored: Vec<(f64, &lsr_core::registry::OperatorSchema)> = ops
        .iter()
        .zip(&docs)
        .map(|(op, (name, summary))| {
            let score = asked
                .iter()
                .zip(&weight)
                .map(|(w, k)| {
                    k * (3.0 * f64::from(u8::from(hit(name, w)))
                        + f64::from(u8::from(hit(summary, w))))
                })
                .sum();
            (score, *op)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.type_id.cmp(&b.1.type_id)));
    let best = scored.first().map_or(0.0, |s| s.0);
    let mut picked = Vec::new();
    let mut shown = 0;
    for (score, op) in scored {
        // The clear matches (a fifth of the best score or more), up to 16,
        // and the core set.
        let core = CORE_OPS.contains(&op.type_id.as_str());
        if !core && (score < best * 0.2 || score <= 0.0 || shown >= 16) {
            continue;
        }
        shown += usize::from(!core);
        picked.push(op);
    }
    picked
}

/// `ops` written compactly (`cap`: long parameter lists are cut, as the
/// standard prompt has always done; without it every parameter is listed).
pub(crate) fn ops_block(ops: &[&lsr_core::registry::OperatorSchema], cap: bool) -> String {
    let mut out = String::from(
        "\nOperators for this request (type: inputs -> outputs, `?` optional; parameters as name kind=default):\n",
    );
    for op in ops {
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
        // A numbered series (`opacity0`, `opacity1` …) is listed once as
        // `opacity0..7`, and long lists are cut: a small model otherwise
        // copies every one into its answer.
        let series = |name: &str| {
            let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
            (stem.len() < name.len()).then_some(stem.to_owned())
        };
        let mut seen = std::collections::BTreeSet::new();
        let mut params: Vec<String> = op
            .params
            .iter()
            .filter(|p| series(&p.name).is_none_or(|stem| seen.insert(stem)))
            .map(|p| {
                let kind = match &p.kind {
                    ParamKind::Float { min, max, .. } => {
                        format!("float[{}..{}]", opt(*min), opt(*max))
                    }
                    ParamKind::Int { min, max, .. } => format!(
                        "int[{}..{}]",
                        opt(min.map(|v| v as f64)),
                        opt(max.map(|v| v as f64))
                    ),
                    ParamKind::Menu { options, .. } => format!("menu({})", options.join("|")),
                    k => format!("{k:?}").to_ascii_lowercase(),
                };
                let name = match series(&p.name) {
                    Some(stem) => {
                        let last = op
                            .params
                            .iter()
                            .filter(|q| series(&q.name).as_deref() == Some(stem.as_str()))
                            .count();
                        format!("{}..{}", p.name, last.saturating_sub(1))
                    }
                    None => p.name.clone(),
                };
                format!("{name} {kind}={}", p.default)
            })
            .collect();
        if cap && params.len() > 14 {
            params.truncate(14);
            params.push("… (get_schema lists the rest)".into());
        }
        let _ = writeln!(
            out,
            "{}: {} -> {}; {} — {}",
            op.type_id,
            sockets(&op.inputs),
            sockets(&op.outputs),
            params.join(", "),
            op.summary.trim()
        );
    }
    out
}

fn opt(v: Option<f64>) -> String {
    v.map_or(String::new(), |v| format!("{v}"))
}

/// How a request went./// How a request went.
#[derive(Debug, Clone, Default)]
pub struct Run {
    pub steps: u32,
    pub tool_calls: u32,
    pub tool_errors: u32,
    pub accepted: bool,
    /// Node errors after the last accept.
    pub errors: Vec<String>,
    pub final_text: String,
    pub llm_error: Option<String>,
}

/// The JSON a planning model answers with: the whole network, operator
/// types limited to the registry's.
fn plan_schema(ws: &dyn Workspace) -> Value {
    let types: Vec<&str> = ws
        .registry()
        .iter()
        .map(|o| o.type_id.as_str())
        .filter(|t| !t.starts_with(lsr_core::pack::TYPE_PREFIX))
        .collect();
    json!({
        "type": "object",
        "properties": {
            "nodes": {"type": "array", "items": {"type": "object", "properties": {
                "name": {"type": "string"},
                "type": {"type": "string", "enum": types},
                // A bounded list (grammars enforce maxItems, not
                // maxProperties), so a looping model cannot run on forever.
                "params": {"type": "array", "maxItems": 8, "items": {"type": "object", "properties": {
                    "name": {"type": "string", "maxLength": 40},
                    "value": {"anyOf": [
                        {"type": "number"}, {"type": "string", "maxLength": 200}, {"type": "boolean"},
                        {"type": "array", "items": {"type": "number"}, "maxItems": 4}
                    ]}
                }, "required": ["name", "value"]}}
            }, "required": ["name", "type", "params"]}, "maxItems": 24},
            // Dotted endpoints: separate `output`/`input` fields were read
            // the wrong way round by small models.
            "wires": {"type": "array", "items": {"type": "object", "properties": {
                "from": {"type": "string", "description": "source node.output, e.g. noise1.out"},
                "to": {"type": "string", "description": "destination node.input, e.g. blur1.in"}
            }, "required": ["from", "to"]}, "maxItems": 48}
        },
        "required": ["nodes", "wires"]
    })
}

const PLAN_RULES: &str = "You design node networks (patches) in Lasair, a real-time visual instrument. \
Answer with the WHOLE network as JSON: {\"nodes\": [{\"name\", \"type\", \"params\": [{\"name\", \"value\"}]}], \"wires\": [{\"from\": \"node.output\", \"to\": \"node.input\"}]}, e.g. {\"from\": \"noise1.out\", \"to\": \"blur1.in\"}. Give only the parameters that differ from their defaults.\n\
Rules:\n\
- Include every node the request names or implies: an effect (blur, mirror, kaleidoscope, level …) needs a source node (noise, ramp, constant, shape …) wired into its image input.\n\
- Wire every node: `from` is `<source node>.<output>` (usually `.out`), `to` is `<destination node>.<input>` with an input the destination operator lists; wire types must match (IMG to IMG, SIG to SIG, GEO to GEO).\n\
- To drive a number parameter from a SIG node, wire the SIG node to the parameter by name: {\"from\": \"lfo1.out\", \"to\": \"blend1.mix\"} (a channel binding). SIG outputs never go into image inputs.\n\
- Use only the parameter names listed for each operator. Animate a number parameter with an expression string such as `0.5 + 0.5 * sin(time)` (native subset only: no lists, strings or Python built-ins); read a SIG node's channel in an expression as `op('lfo1')['lfo']`, using the channel names its description gives (an LFO's channel is `lfo`); or wire a SIG node into an operator's SIG input.\n\
- Names are short identifiers (noise1, blur1). Do not explain.\n";

/// A plan's `params` lists (`[{name, value}]`) as the objects
/// `build_network` takes.
pub(crate) fn plan_to_network(plan: &Value) -> Value {
    let mut net = plan.clone();
    // `noise1.out` → from noise1, output out; `blur1` alone → input `in`.
    for w in net["wires"].as_array_mut().into_iter().flatten() {
        let split = |end: &Value, default: &str| {
            let s = end.as_str().unwrap_or_default();
            match s.rsplit_once('.') {
                Some((node, socket)) if !node.is_empty() && !socket.is_empty() => {
                    (node.to_owned(), socket.to_owned())
                }
                _ => (s.to_owned(), default.to_owned()),
            }
        };
        if w.get("output").is_none() {
            let (node, output) = split(&w["from"], "out");
            w["from"] = json!(node);
            w["output"] = json!(output);
        }
        if w.get("input").is_none() {
            let (node, input) = split(&w["to"], "in");
            w["to"] = json!(node);
            w["input"] = json!(input);
        }
    }
    for n in net["nodes"].as_array_mut().into_iter().flatten() {
        if let Some(list) = n["params"].as_array() {
            let obj: serde_json::Map<String, Value> = list
                .iter()
                .filter_map(|p| Some((p["name"].as_str()?.to_owned(), p["value"].clone())))
                .collect();
            n["params"] = Value::Object(obj);
        }
    }
    net
}

/// Plan-then-build for a model with structured output: the model writes the
/// whole network as JSON, it is staged afresh (so failed rounds never pile
/// up nodes), and the errors go back for a corrected plan.
/// The operators for a request: the lexical matches, and at `full` the
/// semantic hits first, as full schemas.
fn ops_for(ws: &dyn Workspace, request: &str, hits: Option<&Hits>) -> String {
    let lexical = lexical_ops(ws, request);
    let Some(hits) = hits else {
        return ops_block(&lexical, true);
    };
    let mut list: Vec<&lsr_core::registry::OperatorSchema> = hits
        .ops
        .iter()
        .filter_map(|t| ws.registry().get(t))
        .filter(|op| !op.type_id.starts_with(lsr_core::pack::TYPE_PREFIX))
        .collect();
    for op in lexical {
        if !list.iter().any(|o| o.type_id == op.type_id) {
            list.push(op);
        }
    }
    ops_block(&list, false)
}

/// The system prompt of planned mode. The standard text for an empty
/// workspace is byte-for-byte what it has always been; a workspace with a
/// network shows it (all assist levels that plan).
pub(crate) fn plan_system(ws: &dyn Workspace, request: &str, hits: Option<&Hits>) -> String {
    let base = ws.network();
    let existing = if base.is_empty() {
        String::new()
    } else {
        format!(
            "\nThe project already has this network (JSON, in the same format as your answer):\n{}\nYour answer is the WHOLE network after your changes: keep every existing node under its exact name and type unless the request says to change it, and add or change only what the request needs.\n",
            network_json(base)
        )
    };
    let examples = hits.map_or(String::new(), |h| examples_block(&h.examples, true));
    format!(
        "{PLAN_RULES}{existing}{}{examples}",
        ops_for(ws, request, hits)
    )
}

/// The system prompt of the tool loop: the bare rules, or the standard
/// text with the operator list.
pub(crate) fn tools_system(
    ws: &dyn Workspace,
    request: &str,
    level: Assist,
    hits: Option<&Hits>,
) -> String {
    if level == Assist::Bare {
        return BARE_RULES.to_owned();
    }
    let examples = hits.map_or(String::new(), |h| examples_block(&h.examples, false));
    // Bare has `get_network`; the others are shown the network they edit.
    let base = ws.network();
    let existing = if base.is_empty() {
        String::new()
    } else {
        format!(
            "\nThe project already has this network (JSON, to read: parameters as name/value pairs, wires as node.output to node.input; build with the build_network format above):\n{}\nKeep every existing node under its exact name and type unless the request says to change it, and add or change only what the request needs.\n",
            network_json(base)
        )
    };
    format!(
        "{}{}{existing}{examples}",
        system_prompt(ws),
        ops_for(ws, request, hits)
    )
}

/// One reviewer's pass over the network built: what is missing or wrong
/// against the request, from the request and a plain description alone.
fn self_review(llm: &mut dyn Llm, ws: &dyn Workspace, request: &str) -> Option<String> {
    let schema = json!({
        "type": "object",
        "properties": {
            "complete": {"type": "boolean"},
            "problems": {"type": "array", "items": {"type": "string", "maxLength": 300}, "maxItems": 6}
        },
        "required": ["complete", "problems"]
    });
    let messages = [
        json!({"role": "system", "content": REVIEW_RULES}),
        json!({"role": "user", "content": format!(
            "Request:\n{request}\n\nThe network built:\n{}\nWhat is missing or wrong against the request?",
            describe_network(ws.network()))}),
    ];
    let v = llm.json(&messages, &schema)?.ok()?;
    let problems: Vec<&str> = v["problems"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|p| !p.trim().is_empty())
        .collect();
    (v["complete"].as_bool() != Some(true) && !problems.is_empty()).then(|| problems.join("\n"))
}

fn build_planned(
    llm: &mut dyn Llm,
    agent: &mut Agent,
    ws: &mut dyn Workspace,
    request: &str,
    o: &Opts,
) -> Option<Run> {
    let (trace, hits) = (o.trace, o.hits);
    let schema = plan_schema(ws);
    let full = o.level == Assist::Full;
    let system = plan_system(ws, request, hits);
    let mut limit = o.rounds;
    let mut reviewed = !full;
    let mut messages = vec![
        json!({"role": "system", "content": system}),
        json!({"role": "user", "content": request}),
    ];
    let mut run = Run::default();
    let accept = agent.scopes.contains(&crate::lock::Scope::Accept);
    while run.steps < limit {
        run.steps += 1;
        // A degenerate reply (cut off, not JSON) is asked for once more.
        let plan = match llm.json(&messages, &schema)? {
            Ok(p) => p,
            Err(e) if e.contains("not JSON") => match llm.json(&messages, &schema)? {
                Ok(p) => p,
                Err(e) => {
                    run.llm_error = Some(e);
                    break;
                }
            },
            Err(e) => {
                run.llm_error = Some(e);
                break;
            }
        };
        if trace {
            eprintln!("[{}] plan {plan}", run.steps);
        }
        // Full: faults a staging run would only half report are named
        // before anything is staged. On the last round the plan is staged
        // anyway, so the network the model leaves is its best one.
        if full {
            let faults = validate_plan(&plan, ws.network(), ws.registry());
            // A validated plan is a call, so refused / calls stays a ratio.
            run.tool_calls += 1;
            if !faults.is_empty() && run.steps < limit {
                run.tool_errors += faults.len() as u32;
                if trace {
                    eprintln!("[{}] faults {faults:?}", run.steps);
                }
                messages.push(json!({"role": "assistant", "content": plan.to_string()}));
                messages.push(json!({"role": "user", "content": format!(
                    "That network has problems, found before it was staged:\n{}\nAnswer with the corrected WHOLE network as JSON (same names for nodes you keep).",
                    faults.iter().take(12).cloned().collect::<Vec<_>>().join("\n"))}));
                continue;
            }
        }
        // Each round restages the whole plan on the network as accepted so far.
        agent.call(ws, "discard_changes", &json!({}));
        // The plan is the WHOLE network: what it omits is deleted.
        let mut staged_plan = plan_to_network(&plan);
        staged_plan["replace"] = json!(true);
        let out = agent.call(ws, "build_network", &staged_plan);
        run.tool_calls += 1;
        let mut problems: Vec<String> = match &out {
            Output::Json(v) => v["errors"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| e.as_str().map(str::to_owned))
                .collect(),
            other => vec![output_text(other)],
        };
        run.tool_errors += problems.len() as u32;
        if accept && !agent.staged().is_empty() {
            match agent.accept(ws, "Assistant") {
                Ok(errors) => {
                    run.accepted = true;
                    let net = ws.network();
                    run.errors = errors
                        .iter()
                        .map(|(id, e)| {
                            format!("{}: {e}", net.node(*id).map_or("?", |n| n.name.as_str()))
                        })
                        .collect();
                    problems.extend(run.errors.iter().map(|e| format!("node error {e}")));
                }
                Err(e) => problems.push(e),
            }
        }
        // SIG nodes that drive nothing, and channels read from nodes that
        // have none: both leave the patch still.
        let net = ws.network();
        let refs = |sig: &lsr_core::graph::Node| {
            let call = format!("op('{}')", sig.name);
            net.nodes().any(|n| {
                n.params.values().any(|p| {
                    p.expr.as_deref().is_some_and(|e| e.contains(&call))
                        || p.bind.as_ref().is_some_and(|b| b.node == sig.id)
                })
            }) || net.wires().any(|w| w.from.node == sig.id)
        };
        // Outputs (`sig.dmx_out`, `sig.audio_out` …) are ends in themselves.
        for n in net
            .nodes()
            .filter(|n| n.type_id.starts_with("sig.") && !n.type_id.ends_with("_out"))
        {
            if !refs(n) {
                problems.push(format!(
                    "`{}` ({}) drives nothing: wire it into a SIG input, or read it in a parameter expression as op('{}')['<channel>']",
                    n.name, n.type_id, n.name
                ));
            }
        }
        for p in &mut problems {
            if p.contains("no such channel")
                && let Some(name) = p
                    .split("op('")
                    .nth(1)
                    .and_then(|r| r.split('\'').next())
                    .map(str::to_owned)
                && let Some(node) = net.node_by_name(&name)
                && !node.type_id.starts_with("sig.")
            {
                let _ = write!(
                    p,
                    " (`{name}` is a {} node and has no channels; channels come from SIG nodes such as sig.lfo)",
                    node.type_id
                );
            }
        }
        // A node name used as a bare variable in an expression.
        let names: Vec<String> = ws.network().nodes().map(|n| n.name.clone()).collect();
        for p in &mut problems {
            if let Some(name) = p
                .split("name '")
                .nth(1)
                .and_then(|r| r.split('\'').next())
                .map(str::to_owned)
            {
                if names.contains(&name) {
                    let _ = write!(
                        p,
                        " (`{name}` is a node: read its channel as op('{name}')['<channel>'], or a parameter as op('{name}').par.<param>)"
                    );
                } else {
                    let _ = write!(
                        p,
                        " (there is no node `{name}`; add it to the network or use one of: {})",
                        names.join(", ")
                    );
                }
            }
        }
        if trace {
            eprintln!("[{}] problems {problems:?}", run.steps);
        }
        if problems.is_empty() {
            // Full: once the network is accepted clean, a reviewer reads
            // the request and a description of it; what it finds is one
            // more plan round (and a few to repair it).
            if !reviewed && run.accepted {
                reviewed = true;
                if let Some(notes) = self_review(llm, ws, request) {
                    if trace {
                        eprintln!("[{}] review {notes}", run.steps);
                    }
                    limit = limit.max(run.steps + REVIEW_ROUNDS);
                    messages.push(json!({"role": "assistant", "content": plan.to_string()}));
                    messages.push(json!({"role": "user", "content": format!(
                        "A review of that network against the request found:\n{notes}\nAnswer with the corrected WHOLE network as JSON (same names for nodes you keep)."
                    )}));
                    continue;
                }
            }
            break;
        }
        messages.push(json!({"role": "assistant", "content": plan.to_string()}));
        messages.push(json!({"role": "user", "content": format!(
            "That network has problems:\n{}\nAnswer with the corrected WHOLE network as JSON (same names for nodes you keep).",
            problems.join("\n"))}));
    }
    Some(run)
}

/// Runs one request to completion (or `max_steps` model turns) at the
/// standard assist level. A model with structured output plans the
/// network as JSON ([`build_planned`]); any other calls the tools turn by
/// turn.
pub fn build(
    llm: &mut dyn Llm,
    agent: &mut Agent,
    ws: &mut dyn Workspace,
    request: &str,
    max_steps: u32,
) -> Run {
    build_with(
        llm,
        agent,
        ws,
        request,
        max_steps,
        &mut Assistance::standard(),
    )
}

/// What [`build_planned`] needs besides the request.
struct Opts<'a> {
    rounds: u32,
    trace: bool,
    level: Assist,
    hits: Option<&'a Hits>,
}

/// [`build`] at the assist level of `assist` (`bare`: the plain tool loop;
/// `full`: standard plus retrieval and the harness, see [`crate::assist`]).
pub fn build_with(
    llm: &mut dyn Llm,
    agent: &mut Agent,
    ws: &mut dyn Workspace,
    request: &str,
    max_steps: u32,
    assist: &mut Assistance,
) -> Run {
    let trace = std::env::var("LSR_AGENT_TRACE").is_ok_and(|v| v == "1");
    let level = assist.level;
    let hits = assist.search(request);
    if level != Assist::Bare {
        let rounds = match level {
            Assist::Full => max_steps.min(FULL_ROUNDS),
            _ => max_steps.min(4),
        };
        let opts = Opts {
            rounds,
            trace,
            level,
            hits: hits.as_ref(),
        };
        if let Some(run) = build_planned(llm, agent, ws, request, &opts) {
            return run;
        }
    }
    build_tools(llm, agent, ws, request, max_steps, level, hits.as_ref())
}

fn build_tools(
    llm: &mut dyn Llm,
    agent: &mut Agent,
    ws: &mut dyn Workspace,
    request: &str,
    max_steps: u32,
    level: Assist,
    hits: Option<&Hits>,
) -> Run {
    let tools: Vec<Value> = agent
        .tools()
        .iter()
        .map(|t| json!({"type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.schema}}))
        .collect();
    // LSR_AGENT_TRACE=1 prints the operators offered, every tool call and
    // the start of its result.
    let trace = std::env::var("LSR_AGENT_TRACE").is_ok_and(|v| v == "1");
    // Bare: no operator list, no nudges and no repair prompts; the model
    // finds operators with the tools and stops when it has no more calls.
    let bare = level == Assist::Bare;
    let system = tools_system(ws, request, level, hits);
    if trace && !bare {
        eprintln!("{}", ops_for(ws, request, hits));
    }
    let mut messages = vec![
        json!({"role": "system", "content": system}),
        json!({"role": "user", "content": request}),
    ];
    let mut run = Run::default();
    let mut nudged = false;
    let (mut last_failed, mut repeats, mut repairs) = (Vec::<String>::new(), 0, 0);
    while run.steps < max_steps {
        run.steps += 1;
        let reply = match llm.chat(&messages, &tools) {
            Ok(r) => r,
            Err(e) => {
                run.llm_error = Some(e);
                break;
            }
        };
        let mut calls = reply["tool_calls"].as_array().cloned().unwrap_or_default();
        if calls.is_empty() {
            calls = calls_in_text(reply["content"].as_str().unwrap_or_default(), &tools, |n| {
                ws.registry().get(n).is_some()
            });
        }
        let mut assistant =
            json!({"role": "assistant", "content": reply["content"].as_str().unwrap_or_default()});
        let calls: Vec<Value> = calls
            .into_iter()
            .enumerate()
            .map(|(i, mut c)| {
                if c["id"].is_null() {
                    c["id"] = json!(format!("call_{}_{i}", run.steps));
                }
                // Ollama refuses a history whose arguments are not an object.
                if let Some(text) = c["arguments"].as_str() {
                    c["arguments"] = serde_json::from_str(text).unwrap_or(json!({}));
                }
                if !c["arguments"].is_object() {
                    c["arguments"] = json!({});
                }
                c
            })
            .collect();
        if !calls.is_empty() {
            assistant["tool_calls"] = json!(
                calls
                    .iter()
                    .map(|c| json!({
                "id": c["id"], "name": c["name"], "arguments": c["arguments"],
                "function": {"name": c["name"], "arguments": c["arguments"]}}))
                    .collect::<Vec<_>>()
            );
        }
        messages.push(assistant);
        if calls.is_empty() {
            run.final_text = reply["content"].as_str().unwrap_or_default().to_owned();
            if trace {
                eprintln!("[{}] text: {}", run.steps, run.final_text);
            }
            // Built but never accepted: ask once to accept and check.
            if !bare && !agent.staged().is_empty() && !nudged {
                nudged = true;
                messages.push(json!({"role": "user", "content": "Call accept_changes now, then fix any errors it reports."}));
                continue;
            }
            // Stopped with failed calls or node errors: ask for a repair
            // a few times before giving up.
            let mut problems = last_failed.clone();
            problems.extend(run.errors.iter().map(|e| format!("node error {e}")));
            if !bare && !problems.is_empty() && repairs < 3 {
                repairs += 1;
                messages.push(json!({"role": "user", "content": format!(
                    "The request is not done; these failed:\n{}\nFix them with tool calls (use the operator types and node names the errors suggest), then call accept_changes.",
                    problems.join("\n"))}));
                continue;
            }
            break;
        }
        let mut failed = Vec::new();
        for c in &calls {
            let mut name = c["name"].as_str().unwrap_or_default();
            let mut args = c["arguments"].clone();
            if let Some(s) = args.as_str() {
                args = serde_json::from_str(s).unwrap_or(json!({}));
            }
            // An operator type used as the tool name means "make one".
            if ws.registry().get(name).is_some() {
                let params = args.as_object().cloned().unwrap_or_default();
                args = json!({"type": name, "params": params});
                name = "create_node";
            }
            let out = agent.call(ws, name, &args);
            if trace {
                let mut o = output_text(&out);
                o.truncate(160);
                eprintln!("[{}] {name} {args} -> {o}", run.steps);
            }
            run.tool_calls += 1;
            if let Output::Error(e) = &out {
                run.tool_errors += 1;
                failed.push(format!("{name} {args}: {e}"));
            }
            if name == "accept_changes"
                && let Output::Json(v) = &out
            {
                run.accepted = true;
                run.errors = v["errors"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|e| {
                        format!(
                            "{}: {}",
                            e["node"].as_str().unwrap_or("?"),
                            e["error"].as_str().unwrap_or("?")
                        )
                    })
                    .collect();
            }
            let mut text = output_text(&out);
            if text.len() > 6000 {
                text.truncate(6000);
                text.push_str("\n… (truncated)");
            }
            messages.push(
                json!({"role": "tool", "tool_call_id": c["id"], "name": name, "content": text}),
            );
        }
        // A small model can repeat the same failing calls turn after turn:
        // say so plainly, then give up on the third repeat.
        if !bare && !failed.is_empty() && failed == last_failed {
            repeats += 1;
            if repeats >= 3 {
                break;
            }
            messages.push(json!({"role": "user", "content": format!(
                "These calls failed again with the same errors. Do not repeat them; read each error and change the call (use the operator types and node names the errors suggest):\n{}",
                failed.join("\n"))}));
        } else {
            repeats = 0;
        }
        last_failed = failed;
    }
    // Whatever is still staged is applied for the record (the eval judges
    // the network the model left).
    if !agent.staged().is_empty() && agent.scopes.contains(&crate::lock::Scope::Accept) {
        match agent.accept(ws, "Assistant") {
            Ok(errors) => {
                run.accepted = true;
                let net = ws.network();
                run.errors = errors
                    .iter()
                    .map(|(id, e)| {
                        format!("{}: {e}", net.node(*id).map_or("?", |n| n.name.as_str()))
                    })
                    .collect();
            }
            Err(e) => run.errors.push(e),
        }
    }
    run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::{Scope, ShowLock};
    use crate::workspace::NetworkWorkspace;

    /// Both Ollama requests turn thinking off: a thinking model would spend
    /// the reply budget thinking and answer with nothing.
    #[test]
    fn ollama_requests_turn_thinking_off() {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let mut bodies = Vec::new();
            for _ in 0..2 {
                let (s, _) = listener.accept().unwrap();
                let mut r = BufReader::new(s);
                let mut len = 0;
                loop {
                    let mut line = String::new();
                    r.read_line(&mut line).unwrap();
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                    if line == "\r\n" {
                        break;
                    }
                }
                let mut body = vec![0; len];
                r.read_exact(&mut body).unwrap();
                bodies.push(serde_json::from_slice::<Value>(&body).unwrap());
                let reply = r#"{"message":{"content":"{}","tool_calls":[]}}"#;
                write!(
                    r.get_mut(),
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                    reply.len()
                )
                .unwrap();
            }
            bodies
        });
        let mut o = Ollama {
            base: format!("http://127.0.0.1:{port}"),
            model: "m".into(),
        };
        o.chat(&[json!({"role": "user", "content": "hi"})], &[])
            .unwrap();
        o.json(
            &[json!({"role": "user", "content": "hi"})],
            &json!({"type": "object"}),
        )
        .unwrap()
        .unwrap();
        for b in server.join().unwrap() {
            assert_eq!(b["think"], false, "{b}");
        }
    }

    /// HTTPS gets as far as the TLS handshake and fails as an error: the
    /// agent carries a crypto provider (a bare ureq agent panics here).
    #[test]
    fn https_fails_as_an_error_not_a_panic() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = std::io::Write::write_all(&mut s, b"not tls\r\n\r\n");
        });
        let err = post_json(
            &format!("https://127.0.0.1:{port}/v1/messages"),
            &[],
            &json!({}),
        )
        .unwrap_err();
        assert!(err.contains("127.0.0.1"), "{err}");
        server.join().unwrap();
    }

    /// A scripted model: replays tool calls, then stops.
    struct Script(Vec<Value>);

    impl Llm for Script {
        fn chat(&mut self, _: &[Value], _: &[Value]) -> Result<Value, String> {
            Ok(if self.0.is_empty() {
                json!({"content": "done", "tool_calls": []})
            } else {
                self.0.remove(0)
            })
        }
        fn name(&self) -> String {
            "script".into()
        }
    }

    #[test]
    fn tool_calls_written_as_text_are_found() {
        let tools = [
            json!({"function": {"name": "create_node"}}),
            json!({"function": {"name": "connect"}}),
        ];
        let text = "{\"name\": \"create_node\", \"arguments\": {\"type\": \"img.noise\"}}\n\
```json\n{\"name\": \"connect\", \"parameters\": {\"from\": \"a\", \"to\": \"b\"}}\n```\n\
{\"name\": \"rm_rf\", \"arguments\": {}} and {not json";
        let calls = calls_in_text(text, &tools, |n| n == "img.noise");
        assert_eq!(calls.len(), 2, "{calls:?}");
        assert_eq!(calls[0]["arguments"]["type"], "img.noise");
        assert_eq!(calls[1]["name"], "connect");
        assert_eq!(calls[1]["arguments"]["to"], "b");
        assert!(calls_in_text("done, the network is built", &tools, |_| false).is_empty());
    }

    /// A structured-output model: replays plans.
    struct Planner(Vec<Value>, usize);

    impl Llm for Planner {
        fn chat(&mut self, _: &[Value], _: &[Value]) -> Result<Value, String> {
            Err("tools are not used with a planner".into())
        }
        fn name(&self) -> String {
            "planner".into()
        }
        fn json(&mut self, messages: &[Value], schema: &Value) -> Option<Result<Value, String>> {
            assert!(
                schema["properties"]["nodes"]["items"]["properties"]["type"]["enum"]
                    .as_array()
                    .is_some_and(|e| e.iter().any(|t| t == "img.noise"))
            );
            self.1 = messages.len();
            Some(if self.0.is_empty() {
                Err("no more plans".into())
            } else {
                Ok(self.0.remove(0))
            })
        }
    }

    #[test]
    fn a_planning_model_is_restaged_each_round_and_told_its_errors() {
        let mut reg = lsr_core::Registry::default();
        lsr_ops_img::register(&mut reg);
        let mut ws = NetworkWorkspace::new(reg);
        let scopes = Scope::defaults()
            .into_iter()
            .chain([Scope::Accept])
            .collect();
        let mut agent = Agent::new(scopes, ShowLock::default());
        let mut llm = Planner(
            vec![
                // Round 1: a wrong input name; the blur is left unwired.
                json!({"nodes": [{"name": "noise1", "type": "img.noise", "params": []},
                                 {"name": "blur1", "type": "img.blur", "params": [{"name": "size", "value": 3}]}],
                       "wires": [{"from": "noise1.out", "to": "blur1.image"}]}),
                // Round 2: fixed.
                json!({"nodes": [{"name": "noise1", "type": "img.noise", "params": []},
                                 {"name": "blur1", "type": "img.blur", "params": [{"name": "size", "value": 3}]}],
                       "wires": [{"from": "noise1.out", "to": "blur1.in"}]}),
            ],
            0,
        );
        let run = build(&mut llm, &mut agent, &mut ws, "noise, blurred", 40);
        assert_eq!(run.steps, 2, "{run:?}");
        assert!(run.llm_error.is_none(), "{run:?}");
        assert!(
            llm.1 >= 4,
            "the second round saw the first plan and its errors"
        );
        assert_eq!(ws.network.len(), 2, "rebuilt by name, not duplicated");
        assert_eq!(ws.network.wires().count(), 1);
    }

    #[test]
    fn build_network_stages_everything_and_updates_by_name() {
        let mut reg = lsr_core::Registry::default();
        lsr_ops_img::register(&mut reg);
        let mut ws = NetworkWorkspace::new(reg);
        let mut agent = Agent::new(Scope::defaults().into_iter().collect(), ShowLock::default());
        let net = json!({
            "nodes": [
                {"name": "noise1", "type": "img.noise", "params": {"scale": "2 + sin(time)", "octaves": "5"}},
                {"name": "blur1", "type": "img.blur", "params": {"size": 4, "radius": 1}},
                {"name": "bad", "type": "ai.noise"}
            ],
            "wires": [
                {"from": "noise1", "to": "blur1", "input": "in"},
                {"from": "bad", "to": "blur1", "input": "in"}
            ]
        });
        let Output::Json(out) = agent.call(&mut ws, "build_network", &net) else {
            panic!("build_network failed");
        };
        assert_eq!(out["nodes"], json!(["noise1", "blur1"]), "{out}");
        assert_eq!(out["wires"], 1);
        let errors = out["errors"].to_string();
        assert!(errors.contains("did you mean `img.noise`"), "{errors}");
        let view = agent.view(&ws).clone();
        let noise = view.node_by_name("noise1").unwrap();
        assert!(
            noise.params.contains_key("scale"),
            "{errors} {:?}",
            noise.params
        );
        assert_eq!(noise.params["scale"].expr.as_deref(), Some("2 + sin(time)"));
        assert_eq!(noise.params["octaves"].value, json!(5));
        // A SIG node wired to a parameter binds it.
        let mut reg = lsr_core::Registry::default();
        lsr_ops_img::register(&mut reg);
        lsr_ops_sig::register(&mut reg);
        let mut ws2 = NetworkWorkspace::new(reg);
        let mut agent2 = Agent::new(Scope::defaults().into_iter().collect(), ShowLock::default());
        let bound = json!({
            "nodes": [{"name": "lfo1", "type": "sig.lfo"}, {"name": "level1", "type": "img.level"}],
            "wires": [{"from": "lfo1", "output": "out", "to": "level1", "input": "brightness"}]
        });
        let Output::Json(out2) = agent2.call(&mut ws2, "build_network", &bound) else {
            panic!("build_network failed");
        };
        assert_eq!(out2["errors"], json!([]), "{out2}");
        let v2 = agent2.view(&ws2).clone();
        let b = v2.node_by_name("level1").unwrap().params["brightness"]
            .bind
            .clone();
        assert_eq!(b.map(|b| b.node), v2.node_by_name("lfo1").map(|n| n.id));
        // Again with the same names: updated, not duplicated.
        let again =
            json!({"nodes": [{"name": "blur1", "type": "img.blur", "params": {"size": 8}}]});
        agent.call(&mut ws, "build_network", &again);
        let view = agent.view(&ws).clone();
        assert_eq!(view.len(), 2);
        assert_eq!(
            view.node_by_name("blur1").unwrap().params["size"].value,
            json!(8)
        );
    }

    /// A66: expressions and wired channels drive vectors and colours.
    #[test]
    fn build_network_animates_vectors_and_colours() {
        let mut reg = lsr_core::Registry::default();
        lsr_ops_img::register(&mut reg);
        lsr_ops_sig::register(&mut reg);
        let mut ws = NetworkWorkspace::new(reg);
        let mut agent = Agent::new(Scope::defaults().into_iter().collect(), ShowLock::default());
        let net = json!({
            "nodes": [
                {"name": "noise1", "type": "img.noise"},
                {"name": "lfo1", "type": "sig.lfo"},
                {"name": "transform1", "type": "img.transform", "params": {"translate": "0.1 * sin(time)"}},
                {"name": "constant1", "type": "img.constant", "params": {"colour": "0.5 + 0.5 * sin(time)"}}
            ],
            "wires": [
                {"from": "noise1", "to": "transform1", "input": "in"},
                {"from": "lfo1", "output": "out", "to": "transform1", "input": "scale"}
            ]
        });
        let Output::Json(out) = agent.call(&mut ws, "build_network", &net) else {
            panic!("build_network failed");
        };
        assert_eq!(out["errors"], json!([]), "{out}");
        assert_eq!(out["wires"], 2, "{out}");
        let view = agent.view(&ws).clone();
        let t = &view.node_by_name("transform1").unwrap().params;
        assert_eq!(t["translate"].expr.as_deref(), Some("0.1 * sin(time)"));
        assert_eq!(t["translate"].value, json!([0.0, 0.0]), "the fallback");
        assert_eq!(
            t["scale"].bind.as_ref().map(|b| b.node),
            view.node_by_name("lfo1").map(|n| n.id)
        );
        let c = &view.node_by_name("constant1").unwrap().params["colour"];
        assert_eq!(c.expr.as_deref(), Some("0.5 + 0.5 * sin(time)"));
        // A list is still a value.
        let list = json!({"nodes": [{"name": "transform1", "type": "img.transform", "params": {"pivot": "[0.25, 0.75]"}}]});
        let Output::Json(out) = agent.call(&mut ws, "build_network", &list) else {
            panic!("build_network failed");
        };
        assert_eq!(out["errors"], json!([]), "{out}");
        let view = agent.view(&ws).clone();
        let p = &view.node_by_name("transform1").unwrap().params["pivot"];
        assert_eq!(
            (p.value.clone(), p.expr.clone()),
            (json!([0.25, 0.75]), None)
        );
    }

    #[test]
    fn the_loop_applies_tool_calls_and_accepts() {
        let mut reg = lsr_core::Registry::default();
        lsr_ops_img::register(&mut reg);
        let mut ws = NetworkWorkspace::new(reg);
        let scopes = Scope::defaults()
            .into_iter()
            .chain([Scope::Accept])
            .collect();
        let mut agent = Agent::new(scopes, ShowLock::default());
        let call =
            |n: &str, a: Value| json!({"content": "", "tool_calls": [{"name": n, "arguments": a}]});
        let mut llm = Script(vec![
            call(
                "create_node",
                json!({"type": "img.noise", "name": "noise1"}),
            ),
            // Arguments as a JSON string, as some models send them.
            call(
                "create_node",
                json!("{\"type\": \"img.blur\", \"name\": \"blur1\"}"),
            ),
            call(
                "connect",
                json!({"from": "noise1", "to": "blur1", "input": "in"}),
            ),
            call("create_node", json!({"type": "img.nope"})),
        ]);
        let run = build(&mut llm, &mut agent, &mut ws, "noise, blurred", 20);
        assert_eq!(run.tool_calls, 4);
        assert_eq!(run.tool_errors, 1);
        assert!(run.accepted, "nudged to accept");
        assert_eq!(ws.network().len(), 2);
        assert!(system_prompt(&ws).contains("img.noise — "));
    }

    // ---- assist levels ----

    use crate::assist::testkit::tiny_registry;

    fn accepting_agent() -> Agent {
        Agent::new(
            Scope::defaults()
                .into_iter()
                .chain([Scope::Accept])
                .collect(),
            ShowLock::default(),
        )
    }

    /// The standard prompts, byte for byte as they were before assist
    /// levels (the golden files were written by the code of that day), so
    /// results stay comparable.
    #[test]
    fn standard_prompts_are_unchanged_for_an_empty_workspace() {
        let ws = NetworkWorkspace::new(tiny_registry());
        assert_eq!(
            plan_system(&ws, "a blurred noise", None),
            include_str!("../tests/golden/standard_plan_system.txt")
        );
        assert_eq!(
            tools_system(&ws, "a blurred noise", Assist::Standard, None),
            include_str!("../tests/golden/standard_tools_system.txt")
        );
    }

    /// A model that records what it is sent and builds one node.
    struct Spy {
        systems: Vec<String>,
        chats: usize,
        json_calls: usize,
        script: Vec<Value>,
    }

    impl Spy {
        fn new() -> Self {
            Self {
                systems: Vec::new(),
                chats: 0,
                json_calls: 0,
                script: vec![
                    json!({"content": "", "tool_calls": [{"name": "build_network",
                    "arguments": {"nodes": [{"name": "noise1", "type": "img.noise"}], "wires": []}}]}),
                ],
            }
        }
    }

    impl Llm for Spy {
        fn chat(&mut self, messages: &[Value], _: &[Value]) -> Result<Value, String> {
            self.chats += 1;
            self.systems
                .push(messages[0]["content"].as_str().unwrap().to_owned());
            Ok(if self.script.is_empty() {
                json!({"content": "done", "tool_calls": []})
            } else {
                self.script.remove(0)
            })
        }
        fn name(&self) -> String {
            "spy".into()
        }
        fn json(&mut self, _: &[Value], _: &Value) -> Option<Result<Value, String>> {
            self.json_calls += 1;
            None
        }
    }

    #[test]
    fn bare_sends_no_operator_list_and_no_extra_prompts() {
        let mut ws = NetworkWorkspace::new(tiny_registry());
        let mut agent = accepting_agent();
        let mut llm = Spy::new();
        let run = build_with(
            &mut llm,
            &mut agent,
            &mut ws,
            "a noise",
            40,
            &mut Assistance::new(Assist::Bare),
        );
        assert_eq!(llm.json_calls, 0, "no planned mode");
        assert_eq!(llm.systems[0], BARE_RULES);
        assert!(!llm.systems[0].contains("img.noise"), "no operator list");
        assert!(!llm.systems[0].contains("Operators"));
        // Built, never accepted, the model says it is done: bare stops
        // there (standard asks it to accept).
        assert_eq!(llm.chats, 2);
        assert_eq!(run.steps, 2);
        assert_eq!(
            ws.network.len(),
            1,
            "what is staged is applied for the record"
        );

        let mut ws = NetworkWorkspace::new(tiny_registry());
        let mut agent = accepting_agent();
        let mut std_llm = Spy::new();
        build_with(
            &mut std_llm,
            &mut agent,
            &mut ws,
            "a noise",
            40,
            &mut Assistance::standard(),
        );
        assert!(
            std_llm.systems[0].contains("img.noise — "),
            "standard lists operators"
        );
        assert!(std_llm.systems[0].contains("Operators for this request"));
        assert!(
            std_llm.chats > 2,
            "standard nudges to accept: {}",
            std_llm.chats
        );
    }

    fn seeded() -> NetworkWorkspace {
        let mut ws = NetworkWorkspace::new(tiny_registry());
        let mut agent = accepting_agent();
        agent.call(
            &mut ws,
            "build_network",
            &json!({"nodes": [
                {"name": "noise1", "type": "img.noise", "params": {"scale": "2 + sin(time)"}},
                {"name": "blur1", "type": "img.blur", "params": {"size": 9}}],
                "wires": [{"from": "noise1", "to": "blur1", "input": "in"}]}),
        );
        agent.accept(&mut ws, "seed").unwrap();
        ws
    }

    /// Before assist levels the standard planned prompt named the existing
    /// nodes and types only: not their parameters, expressions or wires, so
    /// an edit task ("fix the blur") worked blind. Standard and full now
    /// show the network.
    #[test]
    fn planned_mode_shows_the_current_network_when_there_is_one() {
        let ws = seeded();
        for hits in [None, Some(&Hits::default())] {
            let s = plan_system(&ws, "make the blur softer", hits);
            assert!(s.contains(r#""name":"noise1""#), "{s}");
            assert!(s.contains("2 + sin(time)"), "expression shown: {s}");
            assert!(s.contains(r#""value":9"#), "parameter shown: {s}");
            assert!(
                s.contains(r#"{"from":"noise1.out","to":"blur1.in"}"#),
                "wire shown: {s}"
            );
            assert!(
                s.contains("keep every existing node under its exact name"),
                "{s}"
            );
        }
        // And it is what the model is sent.
        let mut ws = seeded();
        let mut llm = FullLlm::new(vec![json!({"nodes": [], "wires": []})], vec![]);
        build_with(
            &mut llm,
            &mut accepting_agent(),
            &mut ws,
            "make the blur softer",
            40,
            &mut Assistance::standard(),
        );
        let first = &llm.log[0].1[0]["content"];
        assert!(first.as_str().unwrap().contains("blur1"), "{first}");
        // An empty workspace shows nothing of the kind.
        let empty = NetworkWorkspace::new(tiny_registry());
        assert!(!plan_system(&empty, "x", None).contains("already has"));
    }

    /// Structured-output model: replays plans, and reviews.
    struct FullLlm {
        plans: Vec<Value>,
        reviews: Vec<Value>,
        /// (was a review, messages) per call.
        log: Vec<(bool, Vec<Value>)>,
    }

    impl FullLlm {
        fn new(plans: Vec<Value>, reviews: Vec<Value>) -> Self {
            Self {
                plans,
                reviews,
                log: Vec::new(),
            }
        }
    }

    impl Llm for FullLlm {
        fn chat(&mut self, _: &[Value], _: &[Value]) -> Result<Value, String> {
            Err("tools are not used with a planner".into())
        }
        fn name(&self) -> String {
            "full".into()
        }
        fn json(&mut self, messages: &[Value], schema: &Value) -> Option<Result<Value, String>> {
            let review = schema["properties"].get("complete").is_some();
            self.log.push((review, messages.to_vec()));
            let queue = if review {
                &mut self.reviews
            } else {
                &mut self.plans
            };
            Some(if queue.is_empty() {
                Err("nothing scripted".into())
            } else {
                Ok(queue.remove(0))
            })
        }
    }

    fn user_text(m: &[Value]) -> String {
        m.last().unwrap()["content"].as_str().unwrap().to_owned()
    }

    #[test]
    fn full_checks_plans_before_staging_and_reviews_the_result() {
        let mut ws = NetworkWorkspace::new(tiny_registry());
        let mut llm = FullLlm::new(
            vec![
                // Round 1: a parameter that does not exist, and a blur with
                // nothing wired into it.
                json!({"nodes": [{"name": "blur1", "type": "img.blur", "params": [{"name": "radius", "value": 3}]}], "wires": []}),
                // Round 2: right.
                json!({"nodes": [{"name": "noise1", "type": "img.noise", "params": []},
                                 {"name": "blur1", "type": "img.blur", "params": [{"name": "size", "value": 3}]}],
                       "wires": [{"from": "noise1.out", "to": "blur1.in"}]}),
                // Round 3, after the review: it moves.
                json!({"nodes": [{"name": "noise1", "type": "img.noise", "params": []},
                                 {"name": "blur1", "type": "img.blur", "params": [{"name": "size", "value": 3}]},
                                 {"name": "lfo1", "type": "sig.lfo", "params": []}],
                       "wires": [{"from": "noise1.out", "to": "blur1.in"}, {"from": "lfo1.out", "to": "blur1.size"}]}),
            ],
            vec![
                json!({"complete": false, "problems": ["nothing moves; the request asks for motion"]}),
            ],
        );
        let run = build_with(
            &mut llm,
            &mut accepting_agent(),
            &mut ws,
            "a blurred noise that moves",
            40,
            &mut Assistance::new(Assist::Full),
        );
        assert!(run.llm_error.is_none(), "{run:?}");
        assert_eq!(run.steps, 3, "{run:?}");
        assert!(run.accepted);
        assert_eq!(ws.network.len(), 3);

        // The repair message after round 1 names the node and the options.
        let repair = user_text(&llm.log[1].1);
        assert!(repair.contains("found before it was staged"), "{repair}");
        assert!(
            repair.contains("node `blur1` (img.blur): no parameter `radius`"),
            "{repair}"
        );
        assert!(
            repair.contains("its parameters are: size, mode"),
            "{repair}"
        );
        assert!(repair.contains("required input `in`"), "{repair}");

        // The review saw the request and a description of the network, and
        // nothing else.
        let (is_review, msgs) = &llm.log[2];
        assert!(*is_review);
        assert_eq!(msgs.len(), 2);
        let text = user_text(msgs);
        assert!(text.contains("a blurred noise that moves"), "{text}");
        assert!(text.contains("- blur1 (img.blur): size = 3"), "{text}");
        assert!(text.contains("- noise1.out -> blur1.in"), "{text}");

        // What it found went back as one more plan round.
        let (is_review, msgs) = &llm.log[3];
        assert!(!*is_review);
        let text = user_text(msgs);
        assert!(
            text.contains("A review of that network") && text.contains("nothing moves"),
            "{text}"
        );
        // Reviewed once: a clean third round ends it.
        assert_eq!(llm.log.len(), 4);
    }

    #[test]
    fn full_allows_six_repair_rounds_and_standard_three() {
        let bad = json!({"nodes": [{"name": "x", "type": "img.nope", "params": []}], "wires": []});
        for (level, steps) in [(Assist::Full, FULL_ROUNDS), (Assist::Standard, 4)] {
            let mut ws = NetworkWorkspace::new(tiny_registry());
            let mut llm = FullLlm::new(vec![bad.clone(); 12], vec![]);
            let run = build_with(
                &mut llm,
                &mut accepting_agent(),
                &mut ws,
                "anything",
                40,
                &mut Assistance::new(level),
            );
            assert_eq!(run.steps, steps, "{level}");
        }
    }

    #[test]
    fn full_retrieval_goes_into_the_prompt_as_full_schemas_and_examples() {
        let ws = NetworkWorkspace::new(tiny_registry());
        let example = crate::assist::Example {
            id: "demo".into(),
            description: "Blurred noise.".into(),
            network: seeded().network,
        };
        let hits = Hits {
            ops: vec!["sig.lfo".into()],
            examples: vec![example],
        };
        let s = plan_system(&ws, "a blurred noise", Some(&hits));
        // The semantic hit leads, then the lexical ones.
        let lfo = s.find("sig.lfo:").unwrap();
        let blur = s.find("img.blur:").unwrap();
        assert!(lfo < blur, "{s}");
        assert!(s.contains("Worked examples of similar networks"), "{s}");
        assert!(s.contains("Example 1 - Blurred noise."), "{s}");
        assert!(
            s.contains(r#"{"from":"noise1.out","to":"blur1.in"}"#),
            "{s}"
        );
        // The tool loop gets the examples in the shape build_network takes.
        let t = tools_system(&ws, "a blurred noise", Assist::Full, Some(&hits));
        assert!(t.contains(r#""params":{"scale":"2 + sin(time)"}"#), "{t}");
        // Standard text is a prefix of nothing here: no examples without hits.
        assert!(!plan_system(&ws, "a blurred noise", None).contains("Worked examples"));
    }

    /// The tool loop (Anthropic models, models without structured output)
    /// is shown the network it edits at standard and full, not at bare, and
    /// nothing of the kind for an empty workspace.
    #[test]
    fn the_tool_loop_shows_the_current_network_except_at_bare() {
        let ws = seeded();
        for level in [Assist::Standard, Assist::Full] {
            let s = tools_system(&ws, "make the blur softer", level, None);
            assert!(s.contains("already has this network"), "{s}");
            assert!(s.contains(r#""name":"blur1""#), "{s}");
            assert!(s.contains("2 + sin(time)"), "{s}");
            assert!(
                s.contains(r#"{"from":"noise1.out","to":"blur1.in"}"#),
                "{s}"
            );
        }
        let bare = tools_system(&ws, "make the blur softer", Assist::Bare, None);
        assert_eq!(bare, BARE_RULES);
        let empty = NetworkWorkspace::new(tiny_registry());
        assert!(!tools_system(&empty, "x", Assist::Standard, None).contains("already has"));
    }

    fn plan_of(nodes: &[(&str, &str)], wires: &[(&str, &str)]) -> Value {
        json!({
            "nodes": nodes.iter().map(|(n, t)| json!({"name": n, "type": t, "params": []})).collect::<Vec<_>>(),
            "wires": wires.iter().map(|(f, t)| json!({"from": f, "to": t})).collect::<Vec<_>>(),
        })
    }

    /// Runs `plans` (one round each) over `ws` in planned mode.
    fn plan_rounds(ws: &mut NetworkWorkspace, plans: Vec<Value>) -> Run {
        let mut llm = FullLlm::new(plans, vec![]);
        build_with(
            &mut llm,
            &mut accepting_agent(),
            ws,
            "change it",
            1,
            &mut Assistance::standard(),
        )
    }

    /// A plan is the WHOLE network: a node it omits is deleted, a wire it
    /// drops is removed, and kept nodes keep their ids.
    #[test]
    fn a_plan_round_replaces_the_network() {
        // Omitted node.
        let mut ws = seeded();
        let noise = ws.network.node_by_name("noise1").unwrap().id;
        plan_rounds(&mut ws, vec![plan_of(&[("noise1", "img.noise")], &[])]);
        assert!(ws.network.node_by_name("blur1").is_none(), "blur1 is gone");
        assert_eq!(ws.network.node_by_name("noise1").unwrap().id, noise);
        assert_eq!(ws.network.wires().count(), 0, "its wire went with it");
        // Dropped wire, node kept.
        let mut ws = seeded();
        plan_rounds(
            &mut ws,
            vec![plan_of(
                &[("noise1", "img.noise"), ("blur1", "img.blur")],
                &[],
            )],
        );
        assert_eq!(ws.network.len(), 2);
        assert_eq!(ws.network.wires().count(), 0, "the dropped wire is gone");
        // A wire kept stays.
        let mut ws = seeded();
        plan_rounds(
            &mut ws,
            vec![plan_of(
                &[("noise1", "img.noise"), ("blur1", "img.blur")],
                &[("noise1.out", "blur1.in")],
            )],
        );
        assert_eq!(ws.network.wires().count(), 1);
    }

    #[test]
    fn a_plan_round_drops_a_binding_it_does_not_list() {
        let mut reg = lsr_core::Registry::default();
        lsr_ops_img::register(&mut reg);
        lsr_ops_sig::register(&mut reg);
        let mut ws = NetworkWorkspace::new(reg);
        let bound = json!({"nodes": [{"name": "lfo1", "type": "sig.lfo"}, {"name": "level1", "type": "img.level"}],
            "wires": [{"from": "lfo1", "output": "out", "to": "level1", "input": "brightness"}]});
        let mut agent = accepting_agent();
        agent.call(&mut ws, "build_network", &bound);
        agent.accept(&mut ws, "seed").unwrap();
        assert!(
            ws.network.node_by_name("level1").unwrap().params["brightness"]
                .bind
                .is_some()
        );
        plan_rounds(
            &mut ws,
            vec![plan_of(
                &[("lfo1", "sig.lfo"), ("level1", "img.level")],
                &[],
            )],
        );
        let level = ws.network.node_by_name("level1").unwrap();
        assert!(
            level
                .params
                .get("brightness")
                .is_none_or(|p| p.bind.is_none())
        );
    }

    /// Round 2 renames a node: round 1's node is not left behind.
    #[test]
    fn a_second_round_that_renames_a_node_leaves_no_orphan() {
        let mut ws = NetworkWorkspace::new(tiny_registry());
        let mut llm = FullLlm::new(
            vec![
                // Round 1: a wrong input name, so a second round follows.
                plan_of(
                    &[("noise1", "img.noise"), ("blur1", "img.blur")],
                    &[("noise1.out", "blur1.image")],
                ),
                plan_of(
                    &[("clouds", "img.noise"), ("soft", "img.blur")],
                    &[("clouds.out", "soft.in")],
                ),
            ],
            vec![],
        );
        let run = build_with(
            &mut llm,
            &mut accepting_agent(),
            &mut ws,
            "noise, blurred",
            40,
            &mut Assistance::standard(),
        );
        assert_eq!(run.steps, 2, "{run:?}");
        let names: Vec<&str> = ws.network.nodes().map(|n| n.name.as_str()).collect();
        assert_eq!(ws.network.len(), 2, "{names:?}");
        assert!(ws.network.node_by_name("noise1").is_none());
        assert!(ws.network.node_by_name("blur1").is_none());
        assert_eq!(ws.network.wires().count(), 1);
    }

    /// Full counts each validated plan as a call, so refused / calls is a ratio.
    #[test]
    fn full_counts_a_validated_plan_as_a_call() {
        let mut ws = NetworkWorkspace::new(tiny_registry());
        let mut llm = FullLlm::new(
            vec![
                // Faults found before staging: a blur with nothing wired in.
                plan_of(&[("blur1", "img.blur")], &[]),
                plan_of(
                    &[("noise1", "img.noise"), ("blur1", "img.blur")],
                    &[("noise1.out", "blur1.in")],
                ),
            ],
            vec![json!({"complete": true, "problems": []})],
        );
        let run = build_with(
            &mut llm,
            &mut accepting_agent(),
            &mut ws,
            "noise, blurred",
            40,
            &mut Assistance::new(Assist::Full),
        );
        assert!(run.tool_errors >= 1, "{run:?}");
        assert!(run.tool_calls >= run.tool_errors.min(run.steps), "{run:?}");
        assert_eq!(
            run.tool_calls, 3,
            "two validations and one staging: {run:?}"
        );
    }
}
