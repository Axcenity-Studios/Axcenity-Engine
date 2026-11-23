use super::super::tests::{drag_frames, sample_script};
use super::*;

fn number(value: f64) -> Expression {
    Expression::Number { value }
}

fn print(text: &str) -> Block {
    Block::new(BlockKind::Print {
        value: Expression::Text {
            value: text.to_owned(),
        },
    })
}

fn loose(blocks: Vec<Block>) -> Block {
    Block::new(BlockKind::Unattached { body: blocks })
}

fn lua(script: &BlockScript) -> String {
    translate(script).unwrap().source
}

#[test]
fn an_exec_wire_moves_the_chain_and_leaves_what_was_there_loose() {
    let wait = Block::new(BlockKind::Wait {
        seconds: number(1.0),
    });
    let hello = print("hello");
    let (wait_id, hello_id) = (wait.id, hello.id);
    let start = Block::new(BlockKind::WhenStarted { body: vec![wait] });
    let start_id = start.id;
    let mut script = BlockScript {
        blocks: vec![start, loose(vec![hello])],
        ..Default::default()
    };

    connect_exec(&mut script, start_id, Out::Body(0), hello_id).unwrap();
    assert_eq!(
        lua(&script),
        "task.spawn(function()\n    print(\"hello\")\nend)\n"
    );
    // The wait that was wired there is loose now: a node that doesn't run.
    assert!(!has_exec_input(&script, wait_id));
    assert_eq!(script.blocks.len(), 2, "{script:#?}");

    // Then wires the wait after the print.
    connect_exec(&mut script, hello_id, Out::Then, wait_id).unwrap();
    assert_eq!(
        lua(&script),
        "task.spawn(function()\n    print(\"hello\")\n    task.wait(1)\nend)\n"
    );
    assert_eq!(script.blocks.len(), 1, "the emptied loose stack is gone");

    // Breaking the print's input leaves both loose, and nothing runs.
    break_exec_in(&mut script, hello_id);
    assert_eq!(lua(&script), "task.spawn(function()\nend)\n");
    assert!(!has_exec_input(&script, hello_id));
    assert!(
        has_exec_input(&script, wait_id),
        "it's still after the print"
    );
}

#[test]
fn a_wire_that_would_loop_back_is_refused() {
    let first = print("a");
    let second = print("b");
    let (first_id, second_id) = (first.id, second.id);
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![first, second],
        })],
        ..Default::default()
    };
    let before = script.clone();
    assert!(connect_exec(&mut script, second_id, Out::Then, first_id).is_err());
    assert!(connect_exec(&mut script, first_id, Out::Then, first_id).is_err());
    assert_eq!(script, before);
}

#[test]
fn the_false_pin_gives_a_branch_its_else_and_loses_it_again() {
    let branch = Block::new(BlockKind::If {
        condition: Expression::Boolean { value: true },
        body: vec![print("yes")],
    });
    let no = print("no");
    let (branch_id, no_id) = (branch.id, no.id);
    let mut script = BlockScript {
        blocks: vec![
            Block::new(BlockKind::WhenStarted { body: vec![branch] }),
            loose(vec![no]),
        ],
        ..Default::default()
    };
    connect_exec(&mut script, branch_id, Out::Body(1), no_id).unwrap();
    assert!(matches!(
        find_block(&script, branch_id).unwrap().kind,
        BlockKind::IfElse { .. }
    ));
    assert!(
        lua(&script).contains("else\n        print(\"no\")"),
        "{}",
        lua(&script)
    );
    break_exec_out(&mut script, branch_id, Out::Body(1));
    assert!(matches!(
        find_block(&script, branch_id).unwrap().kind,
        BlockKind::If { .. }
    ));
    assert!(!lua(&script).contains("else"));
}

#[test]
fn value_wires_check_the_input_and_copy_a_node_that_is_already_wired() {
    let branch = Block::new(BlockKind::If {
        condition: Expression::Boolean { value: false },
        body: Vec::new(),
    });
    let wait = Block::new(BlockKind::Wait {
        seconds: Expression::Random {
            min: Box::new(number(1.0)),
            max: Box::new(number(3.0)),
        },
    });
    let other_wait = Block::new(BlockKind::Wait {
        seconds: number(0.0),
    });
    let (branch_id, wait_id, other_id) = (branch.id, wait.id, other_wait.id);
    let compare = LooseValue::new(Expression::Compare {
        op: Comparison::Greater,
        left: Box::new(Expression::Variable {
            name: "score".to_owned(),
        }),
        right: Box::new(number(10.0)),
    });
    let compare_id = compare.id;
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![branch, wait, other_wait],
        })],
        tabs: false,
        graph: Graph {
            loose: vec![compare],
            nodes: BTreeMap::from([(compare_id.to_string(), [5.0, 6.0])]),
            ..Default::default()
        },
        ..Default::default()
    };
    let condition = ValueAt {
        owner: branch_id,
        path: vec![0],
    };
    let random = ValueAt {
        owner: wait_id,
        path: vec![0],
    };

    // A number doesn't fit a condition.
    assert!(connect_value(&mut script, &random, &condition).is_err());
    // A loose comparison moves in, and keeps its place.
    connect_value(&mut script, &ValueAt::loose(compare_id), &condition).unwrap();
    assert!(script.graph.loose.is_empty());
    assert_eq!(script.graph.nodes.get(&condition.key()), Some(&[5.0, 6.0]));
    // A wired value is copied: both waits use a random number.
    connect_value(
        &mut script,
        &random,
        &ValueAt {
            owner: other_id,
            path: vec![0],
        },
    )
    .unwrap();
    let source = lua(&script);
    assert!(source.contains("if score > 10 then"), "{source}");
    assert_eq!(source.matches("math.random").count(), 2, "{source}");

    // Breaking a value wire leaves the node loose and the input typed in.
    break_value(&mut script, &condition);
    assert_eq!(script.graph.loose.len(), 1);
    assert!(lua(&script).contains("if false then"), "{}", lua(&script));
}

#[test]
fn deleting_a_node_leaves_what_it_was_wired_to() {
    let first = print("a");
    let branch = Block::new(BlockKind::If {
        condition: Expression::Not {
            value: Box::new(Expression::Boolean { value: false }),
        },
        body: vec![print("inside")],
    });
    let last = print("c");
    let (branch_id, last_id) = (branch.id, last.id);
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![first, branch, last],
        })],
        ..Default::default()
    };
    delete_nodes(&mut script, &[NodeRef::Block(branch_id)]);
    assert_eq!(
        lua(&script),
        "task.spawn(function()\n    print(\"a\")\nend)\n"
    );
    assert!(find_block(&script, branch_id).is_none());
    assert!(
        !has_exec_input(&script, last_id),
        "what came after is loose"
    );
    assert_eq!(script.graph.loose.len(), 1, "the not node stays");
    // The event goes too: its stack stays, loose.
    let event = script.blocks[0].id;
    delete_nodes(&mut script, &[NodeRef::Block(event)]);
    assert_eq!(lua(&script), "");
    assert!(
        script
            .blocks
            .iter()
            .all(|block| matches!(block.kind, BlockKind::Unattached { .. }))
    );
}

#[test]
fn every_block_and_reporter_has_a_node_with_every_pin_named() {
    let mut script = sample_script(&[]);
    let mut seen = 0;
    let clone_ids = clone_identifiers(&script);
    visit_nodes(&mut script, &mut |node, body, _| {
        let (title, _, _, _, layout) = describe(&node, &body, &[], &clone_ids);
        // Verify expressions() / children() count matches Slot count in sections/reporter_parts!
        match &body {
            Body::Block(kind) => {
                assert_eq!(
                    kind.expressions().len(),
                    block_slots(kind).len(),
                    "{title}: expressions() len must match block_slots() len"
                );
            }
            Body::Value(expr) => {
                assert_eq!(
                    expr.children().len(),
                    value_slots(expr).len(),
                    "{title}: children() len must match value_slots() len"
                );
            }
        }
        assert!(!title.is_empty(), "{node:?}");
        for item in &layout.left {
            if let Item::Input { name, .. } | Item::Field { name, .. } = item {
                assert!(!name.is_empty(), "{title}: an unnamed pin");
            }
        }
        let named = match &body {
            Body::Block(kind) => block_pin_names(kind).len(),
            Body::Value(value) => value_pin_names(value).len(),
        };
        let items = layout
            .left
            .iter()
            .filter(|item| !matches!(item, Item::ExecIn))
            .count();
        assert_eq!(named, items, "{title} names {named} of its {items} pins");
        seen += 1;
    });
    assert!(seen > 40, "{seen}");
}

#[test]
fn the_layout_never_changes_the_lua_and_survives_a_save() {
    let mut script = sample_script(&[]);
    let before = lua(&script);
    script.graph.shown = true;
    script
        .graph
        .nodes
        .insert(script.blocks[0].id.to_string(), [10.0, 20.0]);
    script.graph.loose.push(LooseValue::new(number(4.0)));
    assert_eq!(lua(&script), before);
    let text = serde_json::to_string(&script).unwrap();
    let read: BlockScript = serde_json::from_str(&text).unwrap();
    assert_eq!(read, script);
}

#[test]
fn nodes_are_laid_out_without_overlapping() {
    let objects = [ObjectInfo {
        path: "Pad".to_owned(),
        class: ClassName::Part,
        names: Vec::new(),
    }];
    let mut script = sample_script(&objects);
    let ctx = theme::test_context();
    let mut boxes = Vec::new();
    let _ = crate::theme::run(&ctx, Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            let context = BlockContext {
                key: "layout test",
                client: true,
                highlighted: None,
                objects: &objects,
            };
            let geometry = measure_all(ui, &mut script, &context, &[]);
            let sizes = geometry
                .iter()
                .map(|(node, geometry)| (node.clone(), geometry.size))
                .collect();
            let places = layout(&script, &sizes);
            assert_eq!(places.len(), geometry.len(), "every node is placed");
            for (node, at) in places {
                boxes.push((node.clone(), Rect::from_min_size(at, geometry[&node].size)));
            }
        });
    });
    for (index, (node, a)) in boxes.iter().enumerate() {
        for (other, b) in &boxes[index + 1..] {
            assert!(
                !a.shrink(1.0).intersects(b.shrink(1.0)),
                "{node:?} {a:?} overlaps {other:?} {b:?}"
            );
        }
    }
}

fn run_graph(script: &mut BlockScript, frames: Vec<Vec<egui::Event>>, name: &str) {
    let size = vec2(1500.0, 950.0);
    let editor = |script: &mut BlockScript, ui: &mut egui::Ui| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                show_graph(
                    ui,
                    script,
                    BlockContext {
                        key: "graph drag test",
                        client: true,
                        highlighted: None,
                        objects: &[],
                    },
                );
            });
    };
    if let Some(folder) = std::env::var_os("AXCENITY_SNAPSHOTS") {
        let mut replay = script.clone();
        let held = frames[..frames.len() - 2].to_vec();
        let path = std::path::PathBuf::from(folder).join(format!("{name}.png"));
        crate::snapshot::snapshot(&path, size, held, |ui| editor(&mut replay, ui));
    }
    let ctx = theme::test_context();
    for events in frames {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        };
        let _ = crate::theme::run(&ctx, input, |ui| editor(script, ui));
    }
}

/// Where `pin` is on screen in a graph view filling a 1500 by 950 window.
fn pin_on_screen(script: &mut BlockScript, pin: &Pin) -> Pos2 {
    let ctx = theme::test_context();
    let mut found = None;
    let _ = crate::theme::run(&ctx, Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            let context = BlockContext {
                key: "pin test",
                client: true,
                highlighted: None,
                objects: &[],
            };
            let geometry = measure_all(ui, script, &context, &[]);
            let sizes = geometry
                .iter()
                .map(|(node, geometry)| (node.clone(), geometry.size))
                .collect();
            let places = layout(script, &sizes);
            for (node, geometry) in &geometry {
                for (candidate, offset) in &geometry.pins {
                    if candidate == pin {
                        found = Some(places[node] + *offset);
                    }
                }
            }
        });
    });
    let origin = pos2(0.0, TOOLBAR_HEIGHT) + GraphState::default().pan;
    origin + found.expect("the pin is drawn").to_vec2()
}

#[test]
fn dragging_a_wire_from_an_event_to_a_node_wires_them() {
    let hello = print("hello");
    let hello_id = hello.id;
    let start = Block::new(BlockKind::WhenStarted { body: Vec::new() });
    let start_id = start.id;
    let mut script = BlockScript {
        blocks: vec![start, loose(vec![hello])],
        tabs: false,
        graph: Graph {
            shown: true,
            nodes: BTreeMap::from([
                (start_id.to_string(), [40.0, 40.0]),
                (hello_id.to_string(), [420.0, 220.0]),
            ]),
            ..Default::default()
        },
        ..Default::default()
    };
    let from = pin_on_screen(&mut script, &Pin::ExecOut(start_id, Out::Body(0)));
    let to = pin_on_screen(&mut script, &Pin::ExecIn(hello_id));
    run_graph(&mut script, drag_frames(from, to), "graph-wire");
    assert_eq!(
        lua(&script),
        "task.spawn(function()\n    print(\"hello\")\nend)\n",
        "{script:#?}"
    );
}

#[test]
fn the_graph_draws_in_both_themes_without_changing_the_script() {
    let objects = vec![
        ObjectInfo {
            path: "Pad".to_owned(),
            class: ClassName::Part,
            names: Vec::new(),
        },
        ObjectInfo {
            path: "Pad.Chime".to_owned(),
            class: ClassName::Sound,
            names: Vec::new(),
        },
    ];
    let mut script = sample_script(&objects);
    // A small script, as Code switched to Graph would be, with a loose
    // value node.
    script.blocks.truncate(2);
    if let BlockKind::WhenStarted { body } = &mut script.blocks[0].kind {
        body.truncate(6);
    }
    script.graph.shown = true;
    script.graph.loose.push(LooseValue::new(Expression::Join {
        left: Box::new(Expression::Text {
            value: "Hi ".to_owned(),
        }),
        right: Box::new(Expression::Variable {
            name: "score".to_owned(),
        }),
    }));
    let snapshots = std::env::var_os("AXCENITY_SNAPSHOTS").map(std::path::PathBuf::from);
    for (theme, name) in [(egui::Theme::Dark, "dark"), (egui::Theme::Light, "light")] {
        let mut drawn = script.clone();
        let mut show_all = |ui: &mut egui::Ui| {
            let ctx = &ui.ctx().clone();
            ctx.set_theme(theme);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    show_graph(
                        ui,
                        &mut drawn,
                        BlockContext {
                            key: "graph test",
                            client: true,
                            highlighted: None,
                            objects: &objects,
                        },
                    );
                });
        };
        match &snapshots {
            Some(folder) => crate::snapshot::snapshot(
                &folder.join(format!("graph-{name}.png")),
                vec2(1500.0, 950.0),
                vec![Vec::new(); 3],
                &mut show_all,
            ),
            None => {
                let ctx = theme::test_context();
                for _ in 0..3 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1500.0, 950.0))),
                        ..Default::default()
                    };
                    let _ = crate::theme::run(&ctx, input, |ui| show_all(ui));
                }
            }
        }
        assert_eq!(drawn, script, "drawing alone changed the script");
    }
}

/// Frames that type `text` into the focused box and press Enter.
fn type_and_enter(text: &str) -> Vec<Vec<egui::Event>> {
    let enter = |pressed| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Default::default(),
    };
    vec![
        Vec::new(),
        vec![egui::Event::Text(text.to_owned())],
        vec![enter(true)],
        vec![enter(false)],
        Vec::new(),
    ]
}

#[test]
fn the_node_menu_finds_a_node_and_adds_it_where_it_opened() {
    let mut script = BlockScript::default();
    let at = pos2(600.0, 400.0);
    let right = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: Default::default(),
    };
    let mut frames = vec![
        vec![egui::Event::PointerMoved(at)],
        vec![right(true)],
        vec![right(false)],
    ];
    frames.extend(type_and_enter("script starts"));
    run_graph(&mut script, frames, "graph-menu");
    assert_eq!(script.blocks.len(), 1, "{script:#?}");
    assert!(matches!(
        script.blocks[0].kind,
        BlockKind::WhenStarted { .. }
    ));
    let place = script.graph.nodes[&script.blocks[0].id.to_string()];
    let expected = at - pos2(0.0, TOOLBAR_HEIGHT).to_vec2() - GraphState::default().pan;
    assert_eq!(place, [expected.x, expected.y]);
}

#[test]
fn a_wire_let_go_on_the_canvas_makes_a_node_wired_to_it() {
    let start = Block::new(BlockKind::WhenStarted { body: Vec::new() });
    let start_id = start.id;
    let mut script = BlockScript {
        blocks: vec![start],
        tabs: false,
        graph: Graph {
            shown: true,
            nodes: BTreeMap::from([(start_id.to_string(), [40.0, 40.0])]),
            ..Default::default()
        },
        ..Default::default()
    };
    let from = pin_on_screen(&mut script, &Pin::ExecOut(start_id, Out::Body(0)));
    let mut frames = drag_frames(from, from + vec2(300.0, 60.0));
    frames.extend(type_and_enter("wait"));
    run_graph(&mut script, frames, "graph-wire-menu");
    assert_eq!(
        lua(&script),
        "task.spawn(function()\n    task.wait(1)\nend)\n",
        "{script:#?}"
    );
}

/// Typed Luau of the kinds a real script is made of.
const TYPED: &str = r#"local gui = workspace:WaitForChild("Controls")
local waiting = nil

for _, row in gui:GetChildren() do
    row.Activated:Connect(function()
        waiting = row
    end)
end

task.delay(1, function()
    print(waiting)
end)

local KEYS = {
    [Enum.KeyCode.E] = true,
}

game:GetService("UserInputService").InputBegan:Connect(function(input, processed)
    if KEYS[input.KeyCode] and not processed then
        gui.Enabled = not gui.Enabled
    elseif waiting then
        waiting = nil
    end
end)
"#;

/// The block whose kind `pick` finds.
fn block_where(script: &BlockScript, pick: impl Fn(&BlockKind) -> bool) -> u64 {
    let mut found = None;
    script.for_each_block(&mut |block| {
        if found.is_none() && pick(&block.kind) {
            found = Some(block.id);
        }
    });
    found.expect("no such block")
}

#[test]
fn a_typed_script_is_a_graph_of_real_nodes_and_wires() {
    let mut script = axcenity_blocks::from_lua(TYPED).unwrap();
    assert_eq!(lua(&script), TYPED);

    // Every node is wired to something that runs, and none is grey.
    let mut titles = Vec::new();
    let mut unnamed = Vec::new();
    let clone_ids = clone_identifiers(&script);
    visit_nodes(&mut script, &mut |node, body, runs| {
        assert!(runs, "{node:?}");
        let (title, category, _, _, layout) = describe(&node, &body, &[], &clone_ids);
        assert_ne!(title, "Lua", "{node:?}");
        assert!(!matches!(category, Category::Lua) || title != "Lua");
        for item in &layout.left {
            if let Item::Input { name, .. } | Item::Field { name, .. } = item
                && name.is_empty()
            {
                unnamed.push(title.clone());
            }
        }
        titles.push(title);
    });
    assert_eq!(unnamed, Vec::<String>::new());
    for wanted in [
        "Script Runs",
        "Local: gui",
        "WaitForChild",
        "For Each",
        "Connect",
        "On Signal",
        "Branch",
        "Make Table",
    ] {
        assert!(
            titles.iter().any(|title| title == wanted),
            "{wanted}: {titles:?}"
        );
    }

    // A signal's handler and a function argument are exec wires.
    let wires = wiring(&script);
    let connect = block_where(&script, |kind| matches!(kind, BlockKind::Connect { .. }));
    assert!(wires.targets.contains_key(&(connect, Out::Body(0))));
    let delay = block_where(&script, |kind| {
        matches!(kind, BlockKind::CallFunction { .. })
    });
    let kind = find_block(&script, delay).unwrap().kind.clone();
    assert_eq!(
        exec_outs(&kind),
        vec![(Out::Body(0), "Function"), (Out::Then, "")]
    );
    assert!(wires.targets.contains_key(&(delay, Out::Body(0))));

    // Each elseif is the False of the Branch before it.
    let branch = block_where(&script, |kind| matches!(kind, BlockKind::IfElse { .. }));
    assert!(wires.targets.contains_key(&(branch, Out::Body(1))));

    // It draws.
    let ctx = theme::test_context();
    let _ = crate::theme::run(&ctx, Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            let context = BlockContext {
                key: "typed script",
                client: true,
                highlighted: None,
                objects: &[],
            };
            let geometry = measure_all(ui, &mut script, &context, &[]);
            assert!(geometry.len() >= titles.len());
        });
    });
}

#[test]
fn rewiring_a_typed_script_rewrites_its_lua() {
    let mut script = axcenity_blocks::from_lua(TYPED).unwrap();
    let handler = block_where(&script, |kind| matches!(kind, BlockKind::OnSignal { .. }));
    let delay = block_where(&script, |kind| {
        matches!(kind, BlockKind::CallFunction { .. })
    });

    // Wiring the delayed call into the handler moves it and what follows it.
    connect_exec(&mut script, handler, Out::Body(0), delay).unwrap();
    let written = lua(&script);
    let inside = written
        .find("InputBegan:Connect(function(input, processed)\n    task.delay(1, function()")
        .is_some();
    assert!(inside, "{written}");

    // The function in the call is still a place blocks run from.
    let moved = find_block(&script, delay).unwrap();
    assert_eq!(moved.kind.bodies().len(), 1);

    // An input can be added to a call and taken off it again.
    let before = lua(&script);
    resize_inputs(&mut script, &NodeRef::Block(delay), true);
    assert!(lua(&script).contains("end, nil)"), "{}", lua(&script));
    resize_inputs(&mut script, &NodeRef::Block(delay), false);
    assert_eq!(lua(&script), before);
}

#[test]
fn load_around_player_pins_match_player_then_position() {
    let mut kind = BlockKind::RequestStreamAround {
        player: Expression::Variable {
            name: "player".to_owned(),
        },
        position: Expression::Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
    };
    let (_, _, _, _, layout) = describe(
        &NodeRef::Block(1),
        &Body::Block(&mut kind),
        &[],
        &HashMap::new(),
    );
    let inputs: Vec<(&str, usize, SlotKind)> = layout
        .left
        .iter()
        .filter_map(|item| match item {
            Item::Input {
                slot, kind, name, ..
            } => Some((*name, *slot, *kind)),
            _ => None,
        })
        .collect();
    assert_eq!(
        inputs,
        vec![
            ("Player", 0, SlotKind::Any),
            ("Position", 1, SlotKind::Vector),
        ]
    );
}

#[test]
fn load_level_node_height_contains_all_rows_without_clipping() {
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![Block::new(BlockKind::LoadLevel {
                level: Expression::Text {
                    value: "Castle".to_owned(),
                },
                spawn: Expression::Text {
                    value: "Gate".to_owned(),
                },
            })],
        })],
        ..Default::default()
    };
    let load_id = block_where(&script, |kind| matches!(kind, BlockKind::LoadLevel { .. }));
    let ctx = theme::test_context();
    let _ = crate::theme::run(&ctx, Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            let context = BlockContext {
                key: "load level height",
                client: false,
                highlighted: None,
                objects: &[],
            };
            let geometry = measure_all(ui, &mut script, &context, &[]);
            let measured = geometry.get(&NodeRef::Block(load_id)).unwrap();
            assert_eq!(measured.rows.len(), 3, "ExecIn, Level, Spawn");
            let total_rows: f32 = measured.rows.iter().sum();
            assert!(
                measured.size.y >= HEADER + 2.0 + total_rows + 8.0,
                "node height {} must contain header + rows {}",
                measured.size.y,
                HEADER + 2.0 + total_rows + 8.0
            );
        });
    });
}

#[test]
fn clone_output_pin_wires_into_dynamic_object_socket_and_increments_identifiers() {
    let clone1 = Block::new(BlockKind::CloneInto {
        target: "Baseplate".into(),
        parent: ObjectRef::default(),
        count: Some(Expression::Number { value: 1.0 }),
        position: None,
        offset_per_clone: None,
        per_clone_positions: None,
    });
    let clone1_id = clone1.id;
    let clone2 = Block::new(BlockKind::CloneInto {
        target: "Baseplate".into(),
        parent: ObjectRef::default(),
        count: Some(Expression::Number { value: 3.0 }),
        position: Some(Expression::Vector3 {
            x: 0.0,
            y: 5.0,
            z: 0.0,
        }),
        offset_per_clone: Some(Expression::Vector3 {
            x: 5.0,
            y: 0.0,
            z: 0.0,
        }),
        per_clone_positions: None,
    });
    let clone2_id = clone2.id;
    let move_it = Block::new(BlockKind::MoveTo {
        target: "Baseplate".into(),
        position: Expression::Vector3 {
            x: 10.0,
            y: 2.0,
            z: 0.0,
        },
    });
    let move_id = move_it.id;
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![clone1, clone2, move_it],
        })],
        ..Default::default()
    };

    // Wire Clone 1's Cloned Object pin into Move To's Part input socket (slot 0).
    let slot = ValueAt {
        owner: move_id,
        path: vec![0],
    };
    connect_block_output(&mut script, clone1_id, 0, &slot).unwrap();

    let code = lua(&script);
    assert!(
        code.contains("local Baseplate_cloned = workspace.Baseplate:Clone()"),
        "{code}"
    );
    assert!(code.contains("local Baseplate_clones_2 = {}"), "{code}");
    assert!(code.contains("local Baseplate_cloned_2 = nil"), "{code}");
    assert!(
        code.contains(
            "Baseplate_cloned:PivotTo(CFrame.new(Vector3.new(10, 2, 0)) * Baseplate_cloned:GetPivot().Rotation)"
        ),
        "{code}"
    );

    // Clone 2 is a batch clone so it exposes both Baseplate_cloned_2 and Baseplate_clones_2.
    let ids = clone_identifiers(&script);
    let outs = block_value_outs(
        &find_block(&script, clone2_id).unwrap().kind,
        clone2_id,
        &ids,
    );
    assert_eq!(outs.len(), 2);
    assert_eq!(outs[0].lua_name, "Baseplate_cloned_2");
    assert_eq!(outs[1].lua_name, "Baseplate_clones_2");
}

#[test]
fn local_block_hoists_when_wired_downstream_and_uses_singular_name_pin() {
    let local_block = Block::new(BlockKind::Local {
        names: "result".to_owned(),
        values: vec![Expression::Number { value: 42.0 }],
    });
    let local_id = local_block.id;
    assert_eq!(block_title(&local_block.kind), "Local: result");
    assert_eq!(block_pin_names(&local_block.kind), vec!["Name", "Value"]);

    let if_block = Block::new(BlockKind::If {
        condition: Expression::Boolean { value: true },
        body: vec![local_block],
    });
    let print_block = Block::new(BlockKind::Print {
        value: Expression::Number { value: 0.0 },
    });
    let print_id = print_block.id;
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![if_block, print_block],
        })],
        ..Default::default()
    };

    // Wire Local's output pin into the downstream Print outside the If branch.
    let slot = ValueAt {
        owner: print_id,
        path: vec![0],
    };
    connect_block_output(&mut script, local_id, 0, &slot).unwrap();

    let code = lua(&script);
    assert!(
        code.contains(
            "    local result\n    if true then\n        result = 42\n    end\n    print(result)"
        ),
        "{code}"
    );
}

#[test]
fn global_variables_initial_values_and_object_ref_serde_backwards_compat() {
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![
                Block::new(BlockKind::ChangeVariable {
                    name: "sharedScore".to_owned(),
                    by: Expression::Number { value: 5.0 },
                }),
                Block::new(BlockKind::Print {
                    value: Expression::ListContains {
                        list: Box::new(Expression::Variable {
                            name: "bag".to_owned(),
                        }),
                        value: Box::new(Expression::Text {
                            value: "Key".to_owned(),
                        }),
                    },
                }),
            ],
        })],
        variables: vec![VariableDef {
            name: "sharedScore".to_owned(),
            initial: Expression::Number { value: 100.0 },
            global: true,
            watched: true,
        }],
        ..Default::default()
    };
    let code = lua(&script);
    assert!(
        code.contains("if _G.sharedScore == nil then _G.sharedScore = 100 end"),
        "{code}"
    );
    assert!(
        code.contains("_G.sharedScore = (tonumber(_G.sharedScore) or 0) + 5"),
        "{code}"
    );
    assert!(
        code.contains("print(table.find(bag, \"Key\") ~= nil)"),
        "{code}"
    );

    // ObjectRef serializes as plain string when static and as Expression object when dynamic.
    let static_ref = ObjectRef::path("Baseplate");
    let json = serde_json::to_string(&static_ref).unwrap();
    assert_eq!(json, "\"Baseplate\"");
    assert_eq!(
        serde_json::from_str::<ObjectRef>(&json).unwrap(),
        static_ref
    );

    let dyn_ref = ObjectRef::expr(Expression::Variable {
        name: "Baseplate_cloned".to_owned(),
    });
    let dyn_json = serde_json::to_string(&dyn_ref).unwrap();
    assert!(dyn_json.contains("\"variable\""), "{dyn_json}");
    assert_eq!(
        serde_json::from_str::<ObjectRef>(&dyn_json).unwrap(),
        dyn_ref
    );

    // Keep script used.
    script.variables.clear();
}

#[test]
fn phase5_events_and_control_flow_pins_and_codegen() {
    // 1. WhenTouched with Players Only filter, Cooldown, and Player output pin wired to Print.
    let print_player = Block::new(BlockKind::Print {
        value: Expression::Number { value: 0.0 },
    });
    let print_id = print_player.id;
    let touched = Block::new(BlockKind::WhenTouched {
        part: ObjectRef::path("Lava"),
        filter: "Players Only".to_owned(),
        filter_value: None,
        cooldown: Some(Expression::Number { value: 1.5 }),
        body: vec![print_player],
    });
    let touched_id = touched.id;

    // 2. OnSignal with dynamic params ("child, parent") + Connection pin wired to Disconnect.
    let disconnect = Block::new(BlockKind::Disconnect {
        connection: Expression::Nil,
    });
    let disc_id = disconnect.id;
    let on_sig = Block::new(BlockKind::OnSignal {
        signal: Expression::Name {
            name: "workspace.ChildAdded".to_owned(),
        },
        method: "Connect".to_owned(),
        params: "child, parent".to_owned(),
        body: vec![disconnect],
    });
    let sig_id = on_sig.id;

    // 3. Control Flow: RepeatUntil, ForRange with Index output, Sequence, CooldownGate, SpawnAsync, DestroyScript.
    let repeat_until = Block::new(BlockKind::RepeatUntil {
        condition: Expression::Boolean { value: true },
        body: vec![Block::new(BlockKind::Print {
            value: Expression::Text {
                value: "once".to_owned(),
            },
        })],
    });
    let seq = Block::new(BlockKind::Sequence {
        branches: vec![
            vec![Block::new(BlockKind::Print {
                value: Expression::Text {
                    value: "first".to_owned(),
                },
            })],
            vec![Block::new(BlockKind::DestroyScript)],
        ],
    });
    let start = Block::new(BlockKind::WhenStarted {
        body: vec![repeat_until, seq],
    });

    let mut script = BlockScript {
        blocks: vec![touched, on_sig, start],
        ..Default::default()
    };

    // Wire WhenTouched output #1 (Player) -> Print input #0.
    let touched_outs = block_value_outs(&script.blocks[0].kind, touched_id, &HashMap::new());
    assert_eq!(touched_outs.len(), 2);
    assert_eq!(touched_outs[0].label, "Other Part");
    assert_eq!(touched_outs[1].label, "Player");
    connect_block_output(
        &mut script,
        touched_id,
        1,
        &ValueAt {
            owner: print_id,
            path: vec![0],
        },
    )
    .unwrap();

    // Wire OnSignal output #2 (Connection, after child & parent) -> Disconnect input #0.
    let sig_outs = block_value_outs(&script.blocks[1].kind, sig_id, &HashMap::new());
    assert_eq!(sig_outs.len(), 3);
    assert_eq!(sig_outs[0].label, "child");
    assert_eq!(sig_outs[1].label, "parent");
    assert_eq!(sig_outs[2].label, "Connection");
    connect_block_output(
        &mut script,
        sig_id,
        2,
        &ValueAt {
            owner: disc_id,
            path: vec![0],
        },
    )
    .unwrap();

    let code = translate(&script).unwrap().source;
    assert!(
        code.contains("local Players = game:GetService(\"Players\")"),
        "{code}"
    );
    assert!(code.contains("local _connections = {}"), "{code}");
    assert!(
        code.contains(&format!("local _cooldown_{touched_id} = -math.huge")),
        "{code}"
    );
    assert!(
        code.contains("local player = Players:GetPlayerFromCharacter(otherPart.Parent)"),
        "{code}"
    );
    assert!(code.contains("if not player then return end"), "{code}");
    assert!(code.contains("os.clock() - _cooldown_"), "{code}");
    assert!(code.contains("print(player)"), "{code}");
    assert!(
        code.contains(&format!(
            "local conn_{sig_id} = workspace.ChildAdded:Connect(function(child, parent)"
        )),
        "{code}"
    );
    assert!(
        code.contains(&format!("conn_{sig_id}:Disconnect()")),
        "{code}"
    );
    assert!(
        code.contains("repeat\n        print(\"once\")\n        task.wait()\n    until true"),
        "{code}"
    );
    assert!(code.contains("script:Destroy()"), "{code}");
}

#[test]
fn phase6_motion_physics_ai_and_sensing_pins_and_codegen() {
    // 1. Motion/Physics: TurnBy (World), SlideTo (wait + completed), SetGravity, SetAngularVelocity, LaunchPart.
    let turn_world = Block::new(BlockKind::TurnBy {
        target: ObjectRef::path("Door"),
        rotation: Expression::Vector3 {
            x: 0.0,
            y: 90.0,
            z: 0.0,
        },
        space: "World".to_owned(),
    });
    let slide = Block::new(BlockKind::SlideTo {
        target: ObjectRef::path("Door"),
        position: Expression::Vector3 {
            x: 0.0,
            y: 12.0,
            z: 0.0,
        },
        duration: number(1.5),
        easing: "Quad".to_owned(),
        wait_until_done: true,
        completed: vec![print("slid")],
    });
    let gravity = Block::new(BlockKind::SetGravity {
        target: ObjectRef::path("Crate"),
        multiplier: number(0.0),
    });
    let ang_vel = Block::new(BlockKind::SetAngularVelocity {
        target: ObjectRef::path("Crate"),
        velocity: Expression::Vector3 {
            x: 0.0,
            y: 6.0,
            z: 0.0,
        },
    });
    let launch = Block::new(BlockKind::LaunchPart {
        target: ObjectRef::path("Crate"),
        direction: Expression::CameraLookDirection,
        speed: number(60.0),
    });

    // 2. AI, Seats & Splines: WhenSeatVacated (Player pin), WhenAgentSenses (Any, Character pin),
    // WhenAgentLosesPlayer, WhenAgentArrives, WhenSplineReached (Any, Which pin).
    let mut vacated = Block::new(BlockKind::WhenSeatVacated {
        seat: ObjectRef::path("Car.DriverSeat"),
        body: Vec::new(),
    });
    let vacated_id = vacated.id;
    let print_vacated_player = Block::new(BlockKind::Print {
        value: Expression::BlockOutput {
            block: vacated_id,
            output: 1,
            name: "player".to_owned(),
        },
    });
    if let BlockKind::WhenSeatVacated { body, .. } = &mut vacated.kind {
        body.push(print_vacated_player);
    }

    let mut senses = Block::new(BlockKind::WhenAgentSenses {
        agent: ObjectRef::path("Guard.Agent"),
        sense: "Any".to_owned(),
        body: Vec::new(),
    });
    let senses_id = senses.id;
    let print_char = Block::new(BlockKind::Print {
        value: Expression::BlockOutput {
            block: senses_id,
            output: 1,
            name: "character".to_owned(),
        },
    });
    if let BlockKind::WhenAgentSenses { body, .. } = &mut senses.kind {
        body.push(print_char);
    }

    let loses = Block::new(BlockKind::WhenAgentLosesPlayer {
        agent: ObjectRef::path("Guard.Agent"),
        sense: "Sight".to_owned(),
        body: vec![Block::new(BlockKind::StopAgent {
            agent: ObjectRef::path("Guard.Agent"),
        })],
    });
    let arrives = Block::new(BlockKind::WhenAgentArrives {
        agent: ObjectRef::path("Guard.Agent"),
        body: vec![Block::new(BlockKind::ControlSpline {
            follower: ObjectRef::path("Train.Follower"),
            action: "Reverse".to_owned(),
        })],
    });

    // 3. Sensing: unified Raycast (Hit Position + Ignore), TriggeringPlayer, DeltaTime,
    // MouseHitPosition, CameraPosition, NearestPlayerTo, PlayerOfPart, PlayerIsAlive,
    // ParentOf, ChildOf, HasTag, GameTime.
    let ray_hit = Block::new(BlockKind::Print {
        value: Expression::Raycast {
            origin: Box::new(Expression::CameraPosition),
            direction: Box::new(Expression::CameraLookDirection),
            output: "Hit Position".to_owned(),
            ignore: Some(ObjectRef::path("Hero")),
        },
    });
    let sensing_checks = vec![
        Block::new(BlockKind::Print {
            value: Expression::MouseHitPosition,
        }),
        Block::new(BlockKind::Print {
            value: Expression::NearestPlayerTo {
                position: Box::new(Expression::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }),
            },
        }),
        Block::new(BlockKind::Print {
            value: Expression::PlayerOfPart {
                part: ObjectRef::path("Pad"),
            },
        }),
        Block::new(BlockKind::Print {
            value: Expression::PlayerIsAlive {
                player: ObjectRef(Box::new(Expression::TriggeringPlayer)),
            },
        }),
        Block::new(BlockKind::Print {
            value: Expression::ChildOf {
                name: Box::new(Expression::Text {
                    value: "Humanoid".to_owned(),
                }),
                target: ObjectRef(Box::new(Expression::ParentOf {
                    target: ObjectRef::path("Pad"),
                })),
            },
        }),
        Block::new(BlockKind::Print {
            value: Expression::HasTag {
                target: ObjectRef::path("Pad"),
                tag: Box::new(Expression::Text {
                    value: "Hazard".to_owned(),
                }),
            },
        }),
        Block::new(BlockKind::Print {
            value: Expression::GameTime,
        }),
    ];
    let mut start_body = vec![turn_world, slide, gravity, ang_vel, launch, ray_hit];
    start_body.extend(sensing_checks);

    let frame_dt = Block::new(BlockKind::EveryFrame {
        body: vec![Block::new(BlockKind::Print {
            value: Expression::DeltaTime,
        })],
    });

    let script = BlockScript {
        blocks: vec![
            Block::new(BlockKind::WhenStarted { body: start_body }),
            vacated,
            senses,
            loses,
            arrives,
            frame_dt,
        ],
        ..Default::default()
    };

    let code = lua(&script);
    assert!(
        code.contains("workspace.Door:PivotTo(CFrame.new(workspace.Door:GetPivot().Position) * CFrame.Angles(math.rad(0), math.rad(90), math.rad(0)) * workspace.Door:GetPivot().Rotation)"),
        "{code}"
    );
    assert!(code.contains("_tween.Completed:Wait()"), "{code}");
    assert!(code.contains("_vf.Force = Vector3.new(0, workspace.Crate.AssemblyMass * workspace.Gravity * (1 - 0), 0)"), "{code}");
    assert!(
        code.contains("workspace.Crate.AssemblyAngularVelocity = Vector3.new(0, 6, 0)"),
        "{code}"
    );
    assert!(
        code.contains("workspace.Crate.AssemblyLinearVelocity = _dir.Unit * 60"),
        "{code}"
    );
    assert!(
        code.contains("if _prevOccupant and not _curr then"),
        "{code}"
    );
    assert!(
        code.contains("local player = Players:GetPlayerFromCharacter(occupant.Parent)"),
        "{code}"
    );
    assert!(
        code.contains("workspace.Guard.Agent.PlayerSensed:Connect(function(sense)"),
        "{code}"
    );
    assert!(
        code.contains("workspace.Guard.Agent.PlayerLost:Connect(function(sense)"),
        "{code}"
    );
    assert!(
        code.contains("workspace.Guard.Agent.DestinationReached:Connect(function()"),
        "{code}"
    );
    assert!(
        code.contains("workspace.Train.Follower.Speed = -workspace.Train.Follower.Speed"),
        "{code}"
    );
    assert!(code.contains("return _r and _r.Position"), "{code}");
    assert!(
        code.contains("Players.LocalPlayer:GetMouse().Hit.Position"),
        "{code}"
    );
    assert!(
        code.contains("CollectionService:HasTag(workspace.Pad, \"Hazard\")"),
        "{code}"
    );
    assert!(code.contains("workspace.DistributedGameTime"), "{code}");
    assert!(
        code.contains("RunService.Heartbeat:Connect(function(deltaTime)\n    print(deltaTime)"),
        "{code}"
    );
}
