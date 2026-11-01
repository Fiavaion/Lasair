//! `lasair agent` (FR-G4, M5 AC1): the assistant from the command line.
//! `ask` builds one request into a project; `eval` runs the network-
//! generation eval (`docs/milestones/m5/eval-prompts.toml`) on fresh
//! projects and writes a Markdown report.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use lsr_agent::assist::{self, Assist, Assistance, DEFAULT_EMBED_MODEL, OllamaEmbedder, Retriever};
use lsr_agent::assistant::{self, Anthropic, Llm, Ollama};
use lsr_agent::lock::Scope;
use lsr_agent::{Agent, ShowLock, Workspace};

use crate::agent_check::{Prompts, copy_dir};
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
        #[command(flatten)]
        model: ModelArgs,
        /// Evaluate the Python expressions the model writes (they are code
        /// with your user's privileges); without it they keep their stored
        /// values and count as node errors. See docs/python-ops.md.
        #[arg(long)]
        allow_python_ops: bool,
    },
}

#[derive(Debug, clap::Args)]
pub struct ModelArgs {
    /// `ollama` (default, local) or `anthropic` (opt-in; ANTHROPIC_API_KEY,
    /// refused in Sovereign mode unless --allow-internet).
    #[arg(long, default_value = "ollama")]
    pub provider: String,
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
            assistance
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
            model,
            allow_python_ops,
        } => eval(
            &prompts,
            out.as_deref(),
            only.as_deref(),
            &model,
            allow_python_ops,
        ),
    }
}

fn eval(
    path: &Path,
    out: Option<&Path>,
    only: Option<&str>,
    m: &ModelArgs,
    allow_python: bool,
) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let prompts: Prompts = toml_edit::de::from_str(&text)?;
    let mut llm = llm(m)?;
    let tmp = std::env::temp_dir().join(format!("lsr-eval-{}", std::process::id()));
    let registry = crate::project::library().0;
    let mut help = assistance(m, &registry);
    let mut rows = String::new();
    let (mut valid, mut total) = (0, 0);
    let (mut points, mut max_points) = (0, 0);
    let mut score_sum = 0.0;
    let start = Instant::now();
    for p in prompts
        .prompt
        .iter()
        .filter(|p| only.is_none_or(|o| p.id.contains(o)))
    {
        total += 1;
        let dir = tmp.join(format!("{}.lasair", p.id));
        let _ = std::fs::remove_dir_all(&dir);
        if let Some(seed) = &p.seed {
            copy_dir(seed, &dir).with_context(|| format!("seed {}", seed.display()))?;
        }
        let t = Instant::now();
        let mut ws = ServeWorkspace::open(&dir, false, allow_python)?;
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
        let why = &verdict.why;
        let ok = verdict.valid();
        valid += usize::from(ok);
        points += verdict.points;
        max_points += verdict.max;
        let score = 100.0 * verdict.points as f64 / verdict.max.max(1) as f64;
        score_sum += score;
        let mut kinds: Vec<&str> = net.nodes().map(|n| n.type_id.as_str()).collect();
        kinds.sort_unstable();
        kinds.dedup();
        let _ = writeln!(
            rows,
            "| {} | {} | {} | {} | {}/{} | {:.0} s | {} | {}/{} ({score:.0}%) |",
            p.id,
            if ok { "valid" } else { "**invalid**" },
            net.len(),
            kinds.join(", "),
            run.tool_errors,
            run.tool_calls,
            t.elapsed().as_secs_f64(),
            why.join("; ").replace('|', "/"),
            verdict.points,
            verdict.max
        );
        eprintln!(
            "{:>20}: {}",
            p.id,
            if ok {
                format!("valid {}/{}", verdict.points, verdict.max)
            } else {
                format!("{}/{} {}", verdict.points, verdict.max, why.join("; "))
            }
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let pct = 100.0 * valid as f64 / total.max(1) as f64;
    let mean_score = score_sum / total.max(1) as f64;
    let report = format!(
        "# Network-generation eval\n\n**Model:** `{}` · **Assist:** {} · **Prompts:** `{}` · **Valid:** {valid}/{total} (**{pct:.0}%**) · **Score:** {points}/{max_points} points (**{mean_score:.0}%** mean per prompt) · **Time:** {:.0} s · **Max turns:** {}\n\nValid = every check passes: at least `min_nodes` nodes, every required type present, every node cooks without an error (judged on a fresh cook after the assistant stops), and for prompts marked animated something outside the channel nodes moves (an expression or a channel binding on it, or a SIG node wired into it). The hard tier adds wiring paths, parameter values, expressions and bindings, node counts, kept nodes and seed projects (see `agent_check.rs`). Every check is one point; Score = points earned / points possible.\n\n| Prompt | Result | Nodes | Types | Refused calls / calls | Time | Why | Score |\n|---|---|---|---|---|---|---|---|\n{rows}",
        llm.name(),
        m.assist,
        path.display(),
        start.elapsed().as_secs_f64(),
        m.max_steps
    );
    match out {
        Some(o) => std::fs::write(o, &report)?,
        None => print!("{report}"),
    }
    println!(
        "{valid}/{total} valid ({pct:.0}%), score {points}/{max_points} ({mean_score:.0}% mean)"
    );
    Ok(())
}
