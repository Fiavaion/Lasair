//! `lasair agent` (FR-G4, M5 AC1): the assistant from the command line.
//! `ask` builds one request into a project; `eval` runs the network-
//! generation eval (`docs/milestones/m5/eval-prompts.toml`) on fresh
//! projects and writes a Markdown report.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use lsr_agent::assist::{
    self, Assist, Assistance, DEFAULT_EMBED_MODEL, OllamaEmbedder, Retriever, effective_max_turns,
};
use lsr_agent::assistant::{self, Anthropic, Llm, Ollama, OpenAiCompat};
use lsr_agent::lock::Scope;
use lsr_agent::{Agent, ShowLock, Workspace};

use crate::agent_check::{Prompts, Verdict, copy_dir};
use crate::serve::ServeWorkspace;

#[derive(Debug, clap::Subcommand)]
pub enum AgentCommand {
    /// Build a request into a project (changes are accepted and saved).
    Ask {
        project: PathBuf,
        request: String,
        #[command(flatten)]
        model: ModelArgs,
        /// Run the project's Python (ops/*.py operators, Script nodes,
        /// Python expressions); see docs/python-ops.md.
        #[arg(long)]
        allow_python_ops: bool,
    },
    /// Run the network-generation eval and write a report.
    Eval {
        #[arg(long, default_value = "docs/milestones/m5/eval-prompts.toml")]
        prompts: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Only prompts whose id contains this.
        #[arg(long)]
        only: Option<String>,
        /// Keep every network the model built, one project per prompt,
        /// in this folder (replacing what is there).
        #[arg(long)]
        keep: Option<PathBuf>,
        #[command(flatten)]
        model: ModelArgs,
        /// Evaluate the Python expressions the model writes (they are code
        /// with your user's privileges); without it they keep their stored
        /// values and count as node errors. See docs/python-ops.md.
        #[arg(long)]
        allow_python_ops: bool,
    },
    /// Re-score kept projects (`eval --keep`) with the current checker and
    /// no model: each is cooked afresh for its node errors. Applies checker
    /// fixes to past runs.
    Judge {
        #[arg(long, default_value = "docs/milestones/m5/eval-prompts.toml")]
        prompts: PathBuf,
        /// The folder `eval --keep` wrote (one `<id>.lasair` per prompt).
        #[arg(long)]
        kept: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Only prompts whose id contains this.
        #[arg(long)]
        only: Option<String>,
        /// The original eval report: the calls and time columns (and a
        /// model error) are read from it; blank without.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Evaluate the Python expressions in the kept projects.
        #[arg(long)]
        allow_python_ops: bool,
    },
}

#[derive(Debug, clap::Args)]
pub struct ModelArgs {
    /// `ollama` (default, local), `openai` (any OpenAI-compatible server:
    /// vLLM, SGLang, llama.cpp, LM Studio; needs --base-url, key from
    /// OPENAI_API_KEY if set) or `anthropic` (opt-in; ANTHROPIC_API_KEY,
    /// refused in Sovereign mode unless --allow-internet).
    #[arg(long, default_value = "ollama")]
    pub provider: String,
    /// `--provider openai`: the server's API root, e.g.
    /// `http://100.81.126.89:8000/v1`. LAN and Tailscale hosts pass the
    /// egress gate in Sovereign mode.
    #[arg(long)]
    pub base_url: Option<String>,
    #[arg(long, default_value = "qwen2.5-coder:7b")]
    pub model: String,
    /// Model turns per request.
    #[arg(long, default_value_t = 40)]
    pub max_steps: u32,
    /// Turn Sovereign mode off for this run (cloud providers).
    #[arg(long)]
    pub allow_internet: bool,
    /// How much help the model gets: `bare` (the plain tool loop: no
    /// operator list, no retrieval, no repair prompts), `standard` (what
    /// the app does) or `full` (standard plus semantic retrieval of
    /// operator docs and worked examples, plan validation, more repair
    /// rounds and a self-review). See docs/agent.md.
    #[arg(long, default_value = "standard")]
    pub assist: Assist,
    /// `--assist full`: the Ollama embedding model for retrieval.
    #[arg(long, default_value = DEFAULT_EMBED_MODEL)]
    pub embed_model: String,
    /// `--assist full`: the folder of example projects retrieved as worked
    /// examples (never the eval's own seeds and solutions).
    #[arg(long, default_value = "examples")]
    pub examples: PathBuf,
}

/// The help for `m.assist`. At `full` this indexes the operators and
/// examples (cached in the app's cache folder); if the embedding model
/// is unavailable it says so once and retrieval stays lexical.
fn assistance(m: &ModelArgs, registry: &lsr_core::Registry) -> Assistance {
    let assistance = Assistance::new(m.assist);
    if m.assist != Assist::Full {
        return assistance;
    }
    let examples = assist::load_examples(&m.examples, registry);
    let embedder = Box::new(OllamaEmbedder::new(&m.embed_model));
    let dir = lsr_ai::storage::assistant_index_dir();
    match Retriever::build(registry, examples, embedder, Some(&dir)) {
        Ok(r) => assistance.with_retriever(r),
        Err(e) => {
            eprintln!(
                "assist full: cannot build the retrieval index ({e}); using lexical retrieval"
            );
            assistance.without_retriever(e)
        }
    }
}

fn llm(m: &ModelArgs) -> Result<Box<dyn Llm>> {
    lsr_ai::egress::set_sovereign(!m.allow_internet);
    Ok(match m.provider.as_str() {
        "ollama" => Box::new(Ollama::new(&m.model)),
        "anthropic" => Box::new(Anthropic {
            key: std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY is not set")?,
            model: m.model.clone(),
        }),
        "openai" => Box::new(OpenAiCompat {
            base_url: m
                .base_url
                .clone()
                .context("--provider openai needs --base-url (e.g. http://host:8000/v1)")?,
            key: std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|k| !k.is_empty()),
            model: m.model.clone(),
        }),
        other => bail!("unknown provider {other:?}"),
    })
}

fn scopes() -> BTreeSet<Scope> {
    Scope::defaults()
        .into_iter()
        .chain([Scope::Accept])
        .collect()
}

pub fn run(cmd: AgentCommand) -> Result<()> {
    match cmd {
        AgentCommand::Ask {
            project,
            request,
            model,
            allow_python_ops,
        } => {
            let mut llm = llm(&model)?;
            let mut ws = ServeWorkspace::open(&project, true, allow_python_ops)?;
            let lock = ShowLock::load(&project).map_err(|e| anyhow!(e))?;
            let mut agent = Agent::new(scopes(), lock);
            let mut help = assistance(&model, ws.registry());
            let run = assistant::build_with(
                llm.as_mut(),
                &mut agent,
                &mut ws,
                &request,
                model.max_steps,
                &mut help,
            );
            println!(
                "{} steps, {} tool calls ({} refused), {} nodes, errors: {:?}",
                run.steps,
                run.tool_calls,
                run.tool_errors,
                ws.network().len(),
                run.errors
            );
            if let Some(e) = run.llm_error {
                bail!(e);
            }
            Ok(())
        }
        AgentCommand::Eval {
            prompts,
            out,
            only,
            keep,
            model,
            allow_python_ops,
        } => eval(
            &prompts,
            out.as_deref(),
            only.as_deref(),
            keep.as_deref(),
            &model,
            allow_python_ops,
        ),
        AgentCommand::Judge {
            prompts,
            kept,
            out,
            only,
            report,
            allow_python_ops,
        } => judge(
            &prompts,
            &kept,
            out.as_deref(),
            only.as_deref(),
            report.as_deref(),
            allow_python_ops,
        ),
    }
}

/// The report's table rows and totals, one prompt at a time.
#[derive(Default)]
struct Rows {
    text: String,
    valid: usize,
    total: usize,
    points: usize,
    max_points: usize,
    score_sum: f64,
}

impl Rows {
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        id: &str,
        net: &lsr_core::graph::Network,
        verdict: &Verdict,
        why: &[String],
        calls: &str,
        time: &str,
    ) {
        let ok = verdict.valid();
        self.total += 1;
        self.valid += usize::from(ok);
        self.points += verdict.points;
        self.max_points += verdict.max;
        let score = 100.0 * verdict.points as f64 / verdict.max.max(1) as f64;
        self.score_sum += score;
        let mut kinds: Vec<&str> = net.nodes().map(|n| n.type_id.as_str()).collect();
        kinds.sort_unstable();
        kinds.dedup();
        let _ = writeln!(
            self.text,
            "| {id} | {} | {} | {} | {calls} | {time} | {} | {}/{} ({score:.0}%) |",
            if ok { "valid" } else { "**invalid**" },
            net.len(),
            kinds.join(", "),
            why.join("; ").replace('|', "/").replace(['\n', '\r'], " "),
            verdict.points,
            verdict.max
        );
        eprintln!(
            "{id:>20}: {}",
            if ok {
                format!("valid {}/{}", verdict.points, verdict.max)
            } else {
                format!("{}/{} {}", verdict.points, verdict.max, why.join("; "))
            }
        );
    }

    fn pct(&self) -> f64 {
        100.0 * self.valid as f64 / self.total.max(1) as f64
    }

    fn mean_score(&self) -> f64 {
        self.score_sum / self.total.max(1) as f64
    }

    fn summary(&self) -> String {
        format!(
            "{}/{} valid ({:.0}%), score {}/{} ({:.0}% mean)",
            self.valid,
            self.total,
            self.pct(),
            self.points,
            self.max_points,
            self.mean_score()
        )
    }
}

const HOW_SCORED: &str = "Valid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible. A model error makes the run invalid but adds no point to the maximum.";

const TABLE_HEAD: &str = "| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |\n|---|---|---|---|---|---|---|---|";

/// `eval --keep`'s folder: absent, empty, or holding only projects (and
/// whatever else) from a previous keep; refuse anything else instead of
/// deleting it.
fn check_keep_folder(keep: &Path) -> Result<()> {
    if !keep.exists() {
        return Ok(());
    }
    if !keep.is_dir() {
        bail!("--keep {} exists and is not a folder", keep.display());
    }
    let entries: Vec<_> = std::fs::read_dir(keep)?.filter_map(|e| e.ok()).collect();
    let has_projects = entries
        .iter()
        .any(|e| e.path().extension().is_some_and(|x| x == "lasair"));
    if !entries.is_empty() && !has_projects {
        bail!(
            "--keep {} exists and holds no *.lasair projects, so it is not a previous keep folder; \
             refusing to replace it (choose a new folder, or empty this one)",
            keep.display()
        );
    }
    Ok(())
}

/// Replaces the `*.lasair` projects in `keep` with those in `from`.
fn replace_kept(from: &Path, keep: &Path) -> std::io::Result<()> {
    if keep.is_dir() {
        for e in std::fs::read_dir(keep)? {
            let p = e?.path();
            if p.is_dir() && p.extension().is_some_and(|x| x == "lasair") {
                std::fs::remove_dir_all(p)?;
            }
        }
    }
    copy_dir(from, keep)
}

fn eval(
    path: &Path,
    out: Option<&Path>,
    only: Option<&str>,
    keep: Option<&Path>,
    m: &ModelArgs,
    allow_python: bool,
) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let prompts: Prompts = toml_edit::de::from_str(&text)?;
    if let Some(keep) = keep {
        check_keep_folder(keep)?;
    }
    let mut llm = llm(m)?;
    let tmp = std::env::temp_dir().join(format!("lsr-eval-{}", std::process::id()));
    let registry = crate::project::library().0;
    let mut help = assistance(m, &registry);
    let semantic_at_start = help.lexical_reason().is_none();
    let mut fell_back: Option<String> = None;
    let mut rows = Rows::default();
    let start = Instant::now();
    for p in prompts
        .prompt
        .iter()
        .filter(|p| only.is_none_or(|o| p.id.contains(o)))
    {
        let dir = tmp.join(format!("{}.lasair", p.id));
        let _ = std::fs::remove_dir_all(&dir);
        if let Some(seed) = &p.seed {
            copy_dir(seed, &dir).with_context(|| format!("seed {}", seed.display()))?;
        }
        let t = Instant::now();
        // With --keep, every accepted change is written so the project can be read later.
        let mut ws = ServeWorkspace::open(&dir, keep.is_some(), allow_python)?;
        let mut agent = Agent::new(scopes(), ShowLock::default());
        let run = assistant::build_with(
            llm.as_mut(),
            &mut agent,
            &mut ws,
            &p.text,
            m.max_steps,
            &mut help,
        );
        // Judge the network the model left, cooked afresh.
        let errors: Vec<String> = ws.cook_errors().into_iter().map(|e| e.1).collect();
        let net = ws.network();
        let verdict = p.judge(net, &registry, &errors, run.llm_error.as_deref());
        let mut why = verdict.why.clone();
        if help.last_search_was_lexical()
            && let Some(reason) = help.lexical_reason()
        {
            why.push(format!("retrieval was lexical here ({reason})"));
            fell_back.get_or_insert_with(|| p.id.clone());
        }
        rows.add(
            &p.id,
            net,
            &verdict,
            &why,
            &format!("{}/{}", run.tool_errors, run.tool_calls),
            &format!("{:.0} s", t.elapsed().as_secs_f64()),
        );
    }
    if let Some(keep) = keep {
        // The report matters more than the copy: a failed copy is a warning.
        if let Err(e) = replace_kept(&tmp, keep) {
            eprintln!(
                "warning: could not keep the projects in {}: {e}",
                keep.display()
            );
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let retrieval = retrieval_line(
        m.assist,
        help.lexical_reason(),
        semantic_at_start,
        fell_back.as_deref(),
    );
    let turns = effective_max_turns(m.assist, m.max_steps);
    let max_turns = if turns == m.max_steps {
        format!("{turns}")
    } else {
        format!(
            "{turns} (`--max-steps` {}; full adds self-review rounds)",
            m.max_steps
        )
    };
    let report = format!(
        "# Network-generation eval\n\n**Model:** `{}` · **Assist:** {} · **Retrieval:** {retrieval} · **Prompts:** `{}` · **Valid:** {}/{} (**{:.0}%**) · **Score:** {}/{} points (**{:.0}%** mean per prompt) · **Time:** {:.0} s · **Max turns:** {max_turns}\n\n{HOW_SCORED}\n\n{TABLE_HEAD}\n{}",
        llm.name(),
        m.assist,
        path.display(),
        rows.valid,
        rows.total,
        rows.pct(),
        rows.points,
        rows.max_points,
        rows.mean_score(),
        start.elapsed().as_secs_f64(),
        rows.text
    );
    match out {
        Some(o) => std::fs::write(o, &report)?,
        None => print!("{report}"),
    }
    println!("{}", rows.summary());
    Ok(())
}

/// The report header's retrieval state: `now` is why retrieval is lexical
/// at the end of the run (`None`: semantic), `fell_back` the first prompt
/// that ran lexical.
fn retrieval_line(
    assist: Assist,
    now: Option<String>,
    semantic_at_start: bool,
    fell_back: Option<&str>,
) -> String {
    match (assist, now) {
        (Assist::Full, None) => "semantic".to_owned(),
        (Assist::Full, Some(r)) if semantic_at_start => format!(
            "semantic, then lexical from `{}` ({r})",
            fell_back.unwrap_or("?")
        ),
        (Assist::Full, Some(r)) => format!("lexical ({r})"),
        _ => "none (only `--assist full` retrieves)".to_owned(),
    }
}

/// The calls and time columns, and a model error, of each prompt in an
/// earlier report (`eval`'s table).
/// (calls, time, model error) per prompt id.
type Earlier = std::collections::BTreeMap<String, (String, String, Option<String>)>;

fn read_report(path: &Path) -> Result<Earlier> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let mut by_id = std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| l.starts_with("| ")) {
        let cols: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cols.len() < 8 || cols[0] == "Prompt" {
            continue;
        }
        let model_error = cols[6]
            .split("; ")
            .find_map(|w| w.strip_prefix("model error: "))
            .map(str::to_owned);
        by_id.insert(
            cols[0].to_owned(),
            (cols[4].to_owned(), cols[5].to_owned(), model_error),
        );
    }
    Ok(by_id)
}

fn judge(
    path: &Path,
    kept: &Path,
    out: Option<&Path>,
    only: Option<&str>,
    report: Option<&Path>,
    allow_python: bool,
) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let prompts: Prompts = toml_edit::de::from_str(&text)?;
    let original = report.map(read_report).transpose()?.unwrap_or_default();
    let registry = crate::project::library().0;
    let mut rows = Rows::default();
    let mut missing = Vec::new();
    for p in prompts
        .prompt
        .iter()
        .filter(|p| only.is_none_or(|o| p.id.contains(o)))
    {
        let dir = kept.join(format!("{}.lasair", p.id));
        if !dir.join("project.toml").exists() {
            missing.push(p.id.as_str());
            continue;
        }
        let mut ws = ServeWorkspace::open(&dir, false, allow_python)?;
        let errors: Vec<String> = ws.cook_errors().into_iter().map(|e| e.1).collect();
        let (calls, time, model_error) =
            original
                .get(&p.id)
                .cloned()
                .unwrap_or((String::new(), String::new(), None));
        let verdict = p.judge(ws.network(), &registry, &errors, model_error.as_deref());
        rows.add(&p.id, ws.network(), &verdict, &verdict.why, &calls, &time);
    }
    let missing = if missing.is_empty() {
        String::new()
    } else {
        format!("\n\nNo kept project for: {}.", missing.join(", "))
    };
    let report = format!(
        "# Network-generation eval (re-scored)\n\n**Kept projects:** `{}` · **Prompts:** `{}` · **Valid:** {}/{} (**{:.0}%**) · **Score:** {}/{} points (**{:.0}%** mean per prompt)\n\nRe-scored from the kept networks with the current checker; no model ran.\n\n{HOW_SCORED}\n\n{TABLE_HEAD}\n{}{missing}\n",
        kept.display(),
        path.display(),
        rows.valid,
        rows.total,
        rows.pct(),
        rows.points,
        rows.max_points,
        rows.mean_score(),
        rows.text
    );
    match out {
        Some(o) => std::fs::write(o, &report)?,
        None => print!("{report}"),
    }
    println!("{}", rows.summary());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// A reason with a line break (a kernel's compile error) stays on its
    /// row, so the report table keeps one line per prompt.
    #[test]
    fn a_multi_line_reason_stays_on_one_row() {
        let mut rows = Rows::default();
        let v = Verdict {
            points: 1,
            max: 2,
            why: vec![],
            model_error: false,
        };
        let why = ["1 node error(s): float a = mod(x,\n  ^ here\r\n".to_owned()];
        rows.add(
            "p",
            &lsr_core::graph::Network::default(),
            &v,
            &why,
            "0/1",
            "1 s",
        );
        assert_eq!(rows.text.lines().count(), 1, "{}", rows.text);
        assert_eq!(rows.text.matches('|').count(), 9);
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lsr-agent-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_header_says_how_retrieval_went() {
        assert_eq!(retrieval_line(Assist::Full, None, true, None), "semantic");
        assert_eq!(
            retrieval_line(
                Assist::Full,
                Some("embeddings unavailable: down".into()),
                false,
                None
            ),
            "lexical (embeddings unavailable: down)"
        );
        assert_eq!(
            retrieval_line(
                Assist::Full,
                Some("embeddings unavailable: down".into()),
                true,
                Some("h-orbit-cam")
            ),
            "semantic, then lexical from `h-orbit-cam` (embeddings unavailable: down)"
        );
        assert!(retrieval_line(Assist::Standard, None, false, None).starts_with("none"));
    }

    /// `--keep` replaces a previous keep folder but never a folder of
    /// something else.
    #[test]
    fn keep_refuses_a_folder_that_is_not_a_previous_keep_folder() {
        let d = scratch("keep");
        // Absent and empty folders are fine.
        assert!(check_keep_folder(&d.join("new")).is_ok());
        assert!(check_keep_folder(&d).is_ok());
        // Someone's documents: refused.
        std::fs::write(d.join("thesis.txt"), "precious").unwrap();
        let err = check_keep_folder(&d).unwrap_err().to_string();
        assert!(err.contains("not a previous keep folder"), "{err}");
        assert!(d.join("thesis.txt").exists());
        // A file is refused too.
        assert!(check_keep_folder(&d.join("thesis.txt")).is_err());
        // A previous keep folder (it holds projects): fine, and replaced
        // project by project, leaving other files alone.
        let prev = scratch("keep-prev");
        std::fs::create_dir_all(prev.join("a.lasair")).unwrap();
        std::fs::write(prev.join("a.lasair/old.txt"), "x").unwrap();
        std::fs::write(prev.join("notes.md"), "mine").unwrap();
        assert!(check_keep_folder(&prev).is_ok());
        let fresh = scratch("keep-fresh");
        std::fs::create_dir_all(fresh.join("b.lasair")).unwrap();
        replace_kept(&fresh, &prev).unwrap();
        assert!(prev.join("b.lasair").is_dir());
        assert!(
            !prev.join("a.lasair").exists(),
            "the old project is replaced"
        );
        assert!(prev.join("notes.md").exists());
        for d in [d, prev, fresh] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn an_earlier_report_gives_calls_time_and_a_model_error() {
        let d = scratch("report");
        let f = d.join("r.md");
        std::fs::write(
            &f,
            "# x\n\n| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |\n|---|---|---|---|---|---|---|---|\n| p1 | valid | 3 | a, b | 1/9 | 12 s |  | 5/5 (100%) |\n| p2 | **invalid** | 1 | a | 0/0 | 3 s | missing x; model error: rate limit | 1/4 (25%) |\n",
        )
        .unwrap();
        let r = read_report(&f).unwrap();
        assert_eq!(r["p1"], ("1/9".to_owned(), "12 s".to_owned(), None));
        assert_eq!(r["p2"].2.as_deref(), Some("rate limit"));
        let _ = std::fs::remove_dir_all(d);
    }

    /// `judge` re-scores kept projects with no model: the reference
    /// solution scores full points, the seed does not, and calls and time
    /// come from the original report. (Needs a GPU to cook.)
    #[test]
    fn judge_rescores_kept_projects_without_a_model() {
        if lsr_gpu::GpuContext::headless().is_err() {
            return;
        }
        let kept = scratch("judge-kept");
        let hard = root().join("docs/milestones/m5/hard");
        copy_dir(
            &hard.join("solutions/h-stark-poster.lasair"),
            &kept.join("h-stark-poster.lasair"),
        )
        .unwrap();
        copy_dir(
            &hard.join("seeds/h-fix-trails.lasair"),
            &kept.join("h-fix-trails.lasair"),
        )
        .unwrap();
        let orig = kept.join("orig.md");
        std::fs::write(
            &orig,
            "| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |\n|---|---|---|---|---|---|---|---|\n| h-stark-poster | valid | 5 | x | 2/14 | 31 s |  | 1/1 |\n",
        )
        .unwrap();
        let out = kept.join("out.md");
        judge(
            &root().join("docs/milestones/m5/eval-prompts-hard.toml"),
            &kept,
            Some(&out),
            None,
            Some(&orig),
            false,
        )
        .unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        let row = |id: &str| {
            text.lines()
                .find(|l| l.starts_with(&format!("| {id} ")))
                .unwrap()
                .to_owned()
        };
        let poster = row("h-stark-poster");
        assert!(poster.contains("| valid |"), "{poster}");
        assert!(poster.contains("| 2/14 | 31 s |"), "{poster}");
        let trails = row("h-fix-trails");
        assert!(trails.contains("**invalid**"), "{trails}");
        assert!(text.contains("No kept project for:"), "{text}");
        assert!(text.contains("(re-scored)"));
        let _ = std::fs::remove_dir_all(kept);
    }
}
