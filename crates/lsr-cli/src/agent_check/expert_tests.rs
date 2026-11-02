//! The expert tier (`docs/milestones/m5/eval-prompts-expert.toml`): its
//! reference solutions, seeds, right answers built another way, and the
//! "copied numbers" and near-miss networks that must lose points.

use std::path::{Path, PathBuf};

use lsr_agent::Workspace;
use lsr_core::graph::{Edit, Network, Param};
use lsr_core::registry::Registry;
use serde_json::{Value, json};

use super::*;
use crate::serve::ServeWorkspace;

const FILE: &str = "docs/milestones/m5/eval-prompts-expert.toml";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn prompts() -> Vec<Prompt> {
    let text = std::fs::read_to_string(root().join(FILE)).unwrap();
    toml_edit::de::from_str::<Prompts>(&text).unwrap().prompt
}

fn prompt(id: &str) -> Prompt {
    prompts().into_iter().find(|p| p.id == id).expect(id)
}

fn solution_dir(id: &str) -> PathBuf {
    root().join(format!("docs/milestones/m5/expert/solutions/{id}.lasair"))
}

/// Scores the project at `dir` as the eval would; node errors come from a
/// real cook, which needs a GPU (without one they are not judged).
fn score(p: &Prompt, dir: &Path) -> (Verdict, bool) {
    let loaded = crate::project::load_checked(dir).unwrap();
    let net = loaded.project.main().cloned().unwrap();
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "lsr-expert-{}-{}",
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

fn base(dir: &Path) -> (Network, Registry) {
    let loaded = crate::project::load_checked(dir).unwrap();
    (loaded.project.main().cloned().unwrap(), loaded.registry)
}

fn plain(v: Value) -> Param {
    Param {
        value: v,
        expr: None,
        bind: None,
    }
}

fn expr(v: Value, e: &str) -> Param {
    Param {
        value: v,
        expr: Some(e.into()),
        bind: None,
    }
}

/// The reference solution of `id` with `edits` (node, parameter, new
/// setting) applied, judged without a cook.
fn variant(id: &str, edits: &[(&str, &str, Param)]) -> Verdict {
    let (mut net, registry) = base(&solution_dir(id));
    for (node, param, p) in edits {
        let node = net.node_by_name(node).expect(node).id;
        net.apply(
            &registry,
            Edit::SetParam {
                node,
                param: (*param).into(),
                value: Some(p.clone()),
            },
        )
        .unwrap();
    }
    prompt(id).judge(&net, &registry, &[], None)
}

fn lost(v: &Verdict, what: &str) -> bool {
    !v.valid() && v.why.iter().any(|w| w.contains(what))
}

#[test]
fn the_expert_prompts_parse_with_unique_ids() {
    let ps = prompts();
    assert_eq!(ps.len(), 12);
    let mut ids: Vec<&str> = ps.iter().map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 12);
    // No operator type id in the text a model reads.
    for p in &ps {
        assert!(
            !p.text.contains("img.") && !p.text.contains("sig.") && !p.text.contains("dat."),
            "{}",
            p.id
        );
    }
}

#[test]
fn every_expert_solution_scores_full_points_and_is_worth_15_to_30() {
    let mut failures = Vec::new();
    for p in prompts() {
        let (v, _) = score(&p, &solution_dir(&p.id));
        eprintln!("{}: {}/{}", p.id, v.points, v.max);
        if !(15..=30).contains(&v.max) {
            failures.push(format!("{}: worth {} points", p.id, v.max));
        }
        if !v.valid() {
            failures.push(format!("{}: {}/{} {:?}", p.id, v.points, v.max, v.why));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn every_expert_seed_misses_points_but_not_all() {
    let mut seeded = 0;
    for p in prompts() {
        let Some(seed) = &p.seed else { continue };
        seeded += 1;
        let (v, cooked) = score(&p, &root().join(seed));
        eprintln!("{} seed: {}/{}", p.id, v.points, v.max);
        assert!(!v.valid(), "{}: the seed already passes", p.id);
        assert!(v.points > 0, "{}: the seed scores nothing", p.id);
        let loaded = crate::project::load_checked(&root().join(seed)).unwrap();
        let net = loaded.project.main().unwrap();
        for k in &p.keep {
            assert!(net.node_by_name(k).is_some(), "{}: seed lacks {k}", p.id);
        }
        if p.id == "x-debug-tunnel" && cooked {
            assert!(
                v.why.iter().any(|w| w.contains("node error")),
                "{:?}",
                v.why
            );
        }
    }
    assert_eq!(seeded, 5);
}

#[test]
fn each_seeded_fault_is_a_lost_point() {
    let faults: [(&str, &[&str]); 3] = [
        (
            "x-debug-tunnel",
            &[
                "trails fade",
                "titled \"second\"",
                "img.transform rotate",
                "img.hsv_adjust hue",
            ],
        ),
        (
            "x-debug-logo-wobble",
            &["badge is on top", "echo loop", "title is visible", "rotate"],
        ),
        (
            "x-debug-colour-show",
            &[
                "colours cover",
                "sparkles show",
                "cmp.window.width",
                "every 4 s",
            ],
        ),
    ];
    for (id, wants) in faults {
        let p = prompt(id);
        let (net, registry) = base(&root().join(p.seed.as_ref().unwrap()));
        let v = p.judge(&net, &registry, &[], None);
        for fault in wants {
            assert!(
                v.why.iter().any(|w| w.contains(fault)),
                "{id}: {fault}: {:?}",
                v.why
            );
        }
    }
}

// ---- Live links: copied numbers lose, live answers built another way win ----

#[test]
fn x_cue_steps_rejects_copied_numbers_and_a_row_that_does_not_step() {
    let id = "x-cue-steps";
    // Copied from the table: the numbers of one cue, no link.
    let v = variant(
        id,
        &[
            ("lights", "brightness", plain(json!(0.6))),
            ("turn", "rotate", plain(json!(20.0))),
        ],
    );
    assert!(
        lost(&v, "brightness follows") && lost(&v, "img.transform"),
        "{:?}",
        v.why
    );
    // A live link to one fixed row (the hard tier's answer): no stepping.
    let v = variant(
        id,
        &[
            (
                "lights",
                "brightness",
                expr(json!(1.0), "op('cues')['verse', 'level']"),
            ),
            (
                "turn",
                "rotate",
                expr(json!(0.0), "op('cues')['verse', 'spin']"),
            ),
        ],
    );
    assert!(
        lost(&v, "brightness follows") && lost(&v, "img.transform"),
        "{:?}",
        v.why
    );
    // The numbers typed in a list by time: steps, but never reads the table.
    let v = variant(
        id,
        &[(
            "lights",
            "brightness",
            expr(json!(1.0), "[0.3, 0.6, 1, 0.5][int(time / 3) % 4]"),
        )],
    );
    assert!(lost(&v, "brightness follows"), "{:?}", v.why);
    // A step every 2 s instead of 3.
    let v = variant(
        id,
        &[(
            "lights",
            "brightness",
            expr(json!(1.0), "op('cues')[1 + int(time / 2) % 4, 'level']"),
        )],
    );
    assert!(lost(&v, "brightness follows"), "{:?}", v.why);
    // Only three of the four cues.
    let v = variant(
        id,
        &[(
            "lights",
            "brightness",
            expr(json!(1.0), "op('cues')[1 + int(time / 3) % 3, 'level']"),
        )],
    );
    assert!(lost(&v, "brightness follows"), "{:?}", v.why);
}

#[test]
fn x_cue_steps_takes_a_time_expression_and_rows_by_number() {
    let v = variant(
        "x-cue-steps",
        &[
            (
                "lights",
                "brightness",
                expr(json!(1.0), "op('cues')[1 + int(time / 3) % 4, 1]"),
            ),
            (
                "turn",
                "rotate",
                expr(json!(0.0), "op('cues')[1 + (int(time) // 3) % 4, 'spin']"),
            ),
        ],
    );
    assert!(v.valid(), "{:?}", v.why);
}

#[test]
fn x_look_sheet_wants_the_wild_row_read_live() {
    let id = "x-look-sheet";
    let v = variant(
        id,
        &[
            ("tint", "hue", plain(json!(140.0))),
            ("zoom", "scale", plain(json!([1.8, 1.8]))),
            ("soften", "size", plain(json!(24.0))),
        ],
    );
    assert!(
        lost(&v, "img.hsv_adjust") && lost(&v, "img.transform") && lost(&v, "img.blur"),
        "{:?}",
        v.why
    );
    // The calm row instead of the wild one.
    let v = variant(
        id,
        &[(
            "soften",
            "size",
            expr(json!(4.0), "op('looks')['calm', 'blur']"),
        )],
    );
    assert!(lost(&v, "img.blur"), "{:?}", v.why);
    // The number typed into the expression.
    let v = variant(id, &[("soften", "size", expr(json!(4.0), "24 + 0 * time"))]);
    assert!(lost(&v, "img.blur"), "{:?}", v.why);
    // The wild row by its number.
    let v = variant(
        id,
        &[
            ("tint", "hue", expr(json!(0.0), "op('looks')[2, 1]")),
            (
                "zoom",
                "scale",
                expr(json!([1.0, 1.0]), "op('looks')[2, 'zoom']"),
            ),
        ],
    );
    assert!(v.valid(), "{:?}", v.why);
}

#[test]
fn x_tempo_from_table_wants_the_clock_linked_and_a_flash_that_dies_away() {
    let id = "x-tempo-from-table";
    let v = variant(id, &[("clock", "bpm", plain(json!(72.0)))]);
    assert!(lost(&v, "sig.beat"), "{:?}", v.why);
    // Another song's row.
    let v = variant(
        id,
        &[(
            "clock",
            "bpm",
            expr(json!(72.0), "op('setlist')['finale', 'bpm']"),
        )],
    );
    assert!(lost(&v, "sig.beat"), "{:?}", v.why);
    // A rising ramp (dark on the beat, bright just before) is not a flash.
    let v = variant(
        id,
        &[(
            "flash",
            "brightness",
            expr(json!(1.0), "op('clock')['beat_phase']"),
        )],
    );
    assert!(lost(&v, "img.level"), "{:?}", v.why);
    // A flash that never dies away.
    let v = variant(
        id,
        &[(
            "flash",
            "brightness",
            expr(json!(1.0), "1 - 0.5 * op('clock')['beat_phase']"),
        )],
    );
    assert!(lost(&v, "img.level"), "{:?}", v.why);
    // An exponential decay is a flash too.
    let v = variant(
        id,
        &[(
            "flash",
            "brightness",
            expr(json!(1.0), "exp(-4 * op('clock')['beat_phase'])"),
        )],
    );
    assert!(v.valid(), "{:?}", v.why);
}

// ---- Exact mappings ----

#[test]
fn x_orbit_dot_is_a_circle_of_radius_point_three_every_eight_seconds() {
    let id = "x-orbit-dot";
    let v = variant(id, &[("swing_x", "phase", plain(json!(0.0)))]);
    assert!(!v.valid(), "two sines in phase draw a diagonal");
    let v = variant(id, &[("swing_x", "amplitude", plain(json!(0.2)))]);
    assert!(!v.valid(), "radius 0.2");
    let v = variant(
        id,
        &[
            ("swing_x", "frequency", plain(json!(0.25))),
            ("swing_y", "frequency", plain(json!(0.25))),
        ],
    );
    assert!(!v.valid(), "a lap every 4 s");
    let v = variant(id, &[("swing_x", "phase", plain(json!(0.75)))]);
    assert!(!v.valid(), "clockwise");
    let v = variant(
        id,
        &[
            ("swing_x", "phase", plain(json!(0.0))),
            ("swing_y", "phase", plain(json!(0.25))),
        ],
    );
    assert!(!v.valid(), "starts at the top");
    let v = variant(
        id,
        &[
            ("swing_x", "waveform", plain(json!("triangle"))),
            ("swing_y", "waveform", plain(json!("triangle"))),
        ],
    );
    assert!(!v.valid(), "triangle waves are not a circle");
    // The same circle as a Python tuple.
    let (mut net, registry) = base(&solution_dir(id));
    let dot = net.node_by_name("dot").unwrap().id;
    net.apply(
        &registry,
        Edit::SetParam {
            node: dot,
            param: "centre".into(),
            value: Some(expr(
                json!([0.5, 0.5]),
                "(0.5 + 0.3 * cos(time * pi / 4), 0.5 + 0.3 * sin(time * pi / 4))",
            )),
        },
    )
    .unwrap();
    let v = prompt(id).judge(&net, &registry, &[], None);
    assert!(v.valid(), "{:?}", v.why);
}

#[test]
fn x_beat_flash_is_tied_to_126_bpm() {
    let id = "x-beat-flash";
    let v = variant(id, &[("clock", "bpm", plain(json!(120.0)))]);
    assert!(!v.valid(), "{:?}", v.why);
    // 120 BPM written into the colour wheel's formula.
    let v = variant(
        id,
        &[(
            "wheel",
            "hue",
            expr(json!(0.0), "time * 120 / 60 / 16 * 360"),
        )],
    );
    assert!(lost(&v, "img.hsv_adjust"), "{:?}", v.why);
    // A turn every 8 beats.
    let v = variant(
        id,
        &[(
            "wheel",
            "hue",
            expr(json!(0.0), "time * 126 / 60 / 8 * 360"),
        )],
    );
    assert!(lost(&v, "img.hsv_adjust"), "{:?}", v.why);
    // A zoom that starts halfway up.
    let v = variant(
        id,
        &[(
            "zoom",
            "scale",
            expr(
                json!([1.0, 1.0]),
                "1.25 + 0.25 * sin(time * 126 / 60 / 8 * tau)",
            ),
        )],
    );
    assert!(lost(&v, "img.transform"), "{:?}", v.why);
    // A flash that is bright on the beat but only dips to 0.6.
    let v = variant(
        id,
        &[(
            "flash",
            "brightness",
            expr(json!(1.0), "1 - 0.4 * op('clock')['beat_phase']"),
        )],
    );
    assert!(!v.valid(), "{:?}", v.why);
    // A flash written as a time formula with no Beat reading is right too.
    let v = variant(
        id,
        &[(
            "flash",
            "brightness",
            expr(json!(1.0), "1 - (time * 126 / 60) % 1"),
        )],
    );
    assert!(v.valid(), "{:?}", v.why);
}

// ---- Economy ----

#[test]
fn the_economy_prompts_fail_a_slow_turn_and_a_dead_centre_dot() {
    let id = "x-economy-mandala";
    let v = variant(id, &[("fold", "rotation", expr(json!(0.0), "time * 1"))]);
    assert!(!v.valid(), "{:?}", v.why);
    let v = variant(id, &[("fold", "rotation", expr(json!(0.0), "time * 360"))]);
    assert!(!v.valid(), "a turn a second");
    let v = variant(id, &[("fold", "segments", plain(json!(8)))]);
    assert!(!v.valid());
    // The turn on the triangle instead of the fold also turns the mandala.
    let v = variant(
        id,
        &[
            ("fold", "rotation", plain(json!(0.0))),
            ("triangle", "rotate", expr(json!(0.0), "time * 6")),
        ],
    );
    assert!(v.valid(), "{:?}", v.why);

    let id = "x-economy-comet";
    let v = variant(id, &[("mix", "mix", plain(json!(1.0)))]);
    assert!(!v.valid(), "a tail that never fades");
    let v = variant(id, &[("swirl", "rotate", plain(json!(0.0)))]);
    assert!(!v.valid(), "a tail that does not swirl");
    let v = variant(id, &[("dot", "centre", plain(json!([0.5, 0.5])))]);
    assert!(!v.valid(), "a dot dead centre");
}

#[test]
fn x_debug_colour_show_takes_a_breath_written_as_a_formula_but_not_a_faster_one() {
    let id = "x-debug-colour-show";
    let (mut net, registry) = base(&solution_dir(id));
    let dim = net.node_by_name("dim").unwrap().id;
    let mut set = |e: &str| {
        net.apply(
            &registry,
            Edit::SetParam {
                node: dim,
                param: "brightness".into(),
                value: Some(expr(json!(1.0), e)),
            },
        )
        .unwrap();
        prompt(id).judge(&net, &registry, &[], None)
    };
    // 0.4 to 1.0 once every 4 s.
    assert!(set("0.7 + 0.3 * sin(time * pi / 2)").valid());
    // Twice as fast (every 2 s), a breath of the wrong depth, and no breath.
    assert!(lost(&set("0.7 + 0.3 * sin(time * pi)"), "slow breath"));
    assert!(lost(&set("0.8 + 0.2 * sin(time * pi / 2)"), "slow breath"));
    assert!(lost(&set("0.7"), "slow breath"));
}

#[test]
fn a_sixth_node_loses_the_economy_budget() {
    let p = prompt("x-economy-comet");
    let (mut net, registry) = base(&solution_dir("x-economy-comet"));
    let id = net.alloc_id();
    net.apply(
        &registry,
        Edit::AddNode {
            node: Node::new(id, "img.level", "extra"),
        },
    )
    .unwrap();
    let v = p.judge(&net, &registry, &[], None);
    assert!(lost(&v, "nodes > 5"), "{:?}", v.why);
}

/// Networks models built that answer a prompt another right way (a pulsing
/// size written as an expression, colour before the fold, a hue that goes
/// round the wheel twice, a Level opacity fade, ...): `<id>--<what>.lasair`.
#[test]
fn right_answers_the_models_found_score_full_points() {
    let dir = root().join("docs/milestones/m5/expert/alternatives");
    let mut seen = 0;
    let mut failures = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap() {
        let path = e.unwrap().path();
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let (v, _) = score(&prompt(name.split("--").next().unwrap()), &path);
        seen += 1;
        if !v.valid() {
            failures.push(format!("{name}: {}/{} {:?}", v.points, v.max, v.why));
        }
    }
    assert!(seen >= 6);
    assert!(failures.is_empty(), "{failures:#?}");
}
