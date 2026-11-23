use super::*;

#[test]
fn server_palettes_leave_out_client_only_blocks() {
    for category in Category::ALL {
        for item in palette(category, false, &[], &[]) {
            match item {
                PaletteItem::Block(kind) => assert!(!kind.client_only(), "{kind:?}"),
                PaletteItem::Reporter(expression) => {
                    assert!(!expression.client_only(), "{expression:?}");
                }
                PaletteItem::NewVariable => {}
            }
        }
    }
    let client: Vec<_> = palette(Category::Events, true, &[], &[])
        .into_iter()
        .filter_map(|item| match item {
            PaletteItem::Block(kind) => Some(*kind),
            _ => None,
        })
        .collect();
    assert!(client.iter().any(BlockKind::client_only));
}

#[test]
fn palette_blocks_name_the_first_object_of_their_kind() {
    let objects = [
        ObjectInfo {
            path: "Floor".to_owned(),
            class: ClassName::Part,
            names: Vec::new(),
        },
        ObjectInfo {
            path: "Floor.Chime".to_owned(),
            class: ClassName::Sound,
            names: Vec::new(),
        },
    ];
    let sound = palette(Category::Sound, false, &objects, &[]);
    assert!(matches!(
        &sound[0],
        PaletteItem::Block(kind) if matches!(&**kind, BlockKind::PlaySound { target, .. } if target.as_path() == Some("Floor.Chime"))
    ));
}

#[test]
fn slots_take_only_reporters_of_their_shape() {
    assert!(!SlotKind::Condition.accepts(BlockShape::Value));
    assert!(SlotKind::Condition.accepts(BlockShape::Boolean));
    assert!(SlotKind::Number.accepts(BlockShape::Boolean));
    assert!(!SlotKind::Number.accepts(BlockShape::Statement));
}

#[test]
fn dropping_a_reporter_and_stacks_puts_them_where_aimed() {
    let mut script = BlockScript {
        blocks: vec![Block::new(BlockKind::WhenStarted {
            body: vec![Block::new(BlockKind::Wait {
                seconds: Expression::Number { value: 1.0 },
            })],
        })],
        ..Default::default()
    };
    let wait = script.blocks[0].kind.bodies()[0][0].id;
    let address = SlotAddress {
        block: wait,
        path: vec![0],
    };
    *address.resolve(&mut script).unwrap() = Expression::Random {
        min: Box::new(Expression::Number { value: 1.0 }),
        max: Box::new(Expression::Number { value: 3.0 }),
    };
    *address.child(1).resolve(&mut script).unwrap() = Expression::Number { value: 9.0 };
    let source = translate(&script).unwrap().source;
    assert!(
        source.contains("task.wait(math.random(math.floor(1), math.floor(9)))"),
        "{source}"
    );

    let hat = script.blocks[0].id;
    drop_stack(
        &mut script,
        StackSpot::Mouth {
            block: hat,
            mouth: 0,
            empty: false,
        },
        vec![Block::new(BlockKind::StopScript)],
        0.0,
    );
    drop_stack(
        &mut script,
        StackSpot::After {
            block: wait,
            last: true,
        },
        vec![Block::new(BlockKind::Print {
            value: Expression::Text {
                value: "done".to_owned(),
            },
        })],
        0.0,
    );
    let source = translate(&script).unwrap().source;
    let order: Vec<_> = ["do return end", "task.wait", "print(\"done\")"]
        .iter()
        .map(|code| source.find(code).unwrap())
        .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{source}");
}

/// A script with every palette block and reporter in it.
pub(super) fn sample_script(objects: &[ObjectInfo]) -> BlockScript {
    let mut events = Vec::new();
    let mut statements = Vec::new();
    let mut reporters = Vec::new();
    for category in Category::ALL {
        for item in palette(category, true, objects, &["score".to_owned()]) {
            match item {
                PaletteItem::Block(kind) if kind.is_event() => events.push(Block::new(*kind)),
                PaletteItem::Block(kind) => statements.push(Block::new(*kind)),
                PaletteItem::Reporter(expression) => reporters.push(expression),
                PaletteItem::NewVariable => {}
            }
        }
    }
    let mut blocks = Vec::new();
    let mut first = events.remove(0);
    first.kind.bodies_mut()[0].extend(statements);
    blocks.push(first);
    let mut touched = events.remove(0);
    touched.position = Some([520.0, 20.0]);
    touched.kind.bodies_mut()[0].extend(reporters.into_iter().map(|value| {
        if value.shape() == BlockShape::Boolean {
            Block::new(BlockKind::If {
                condition: value,
                body: vec![Block::new(BlockKind::StopScript)],
            })
        } else {
            Block::new(BlockKind::Print { value })
        }
    }));
    blocks.push(touched);
    blocks.push(Block {
        position: Some([520.0, 700.0]),
        ..Block::new(BlockKind::Unattached {
            body: vec![Block::new(BlockKind::Wait {
                seconds: Expression::Number { value: 2.5 },
            })],
        })
    });
    // Lua with no blocks, from a script switched from Code.
    blocks.push(Block {
        position: Some([800.0, 20.0]),
        ..Block::new(BlockKind::TopCode {
            code: "local function double(x)
return x * 2
end"
            .to_owned(),
        })
    });
    blocks.push(Block {
        position: Some([800.0, 180.0]),
        ..Block::new(BlockKind::WhenStarted {
            body: vec![
                Block::new(BlockKind::Code {
                    code: "for i = 1, 3 do
print(i)
end"
                    .to_owned(),
                }),
                Block::new(BlockKind::Print {
                    value: Expression::Code {
                        code: "double(4)".to_owned(),
                    },
                }),
            ],
        })
    });
    BlockScript {
        blocks,
        ..Default::default()
    }
}

#[test]
fn every_block_draws_in_both_themes_without_changing_the_script() {
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
    let script = sample_script(&objects);
    let snapshots = std::env::var_os("AXCENITY_SNAPSHOTS").map(std::path::PathBuf::from);
    for (theme, name) in [(egui::Theme::Dark, "dark"), (egui::Theme::Light, "light")] {
        let mut drawn = script.clone();
        let mut show_all = |ui: &mut egui::Ui| {
            let ctx = &ui.ctx().clone();
            ctx.set_theme(theme);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    show(
                        ui,
                        &mut drawn,
                        BlockContext {
                            key: "test",
                            client: true,
                            highlighted: None,
                            objects: &objects,
                        },
                    );
                });
        };
        match &snapshots {
            Some(folder) => crate::snapshot::snapshot(
                &folder.join(format!("blocks-{name}.png")),
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

/// The frames of dragging with the left mouse button from `from` to
/// `to`, in a few steps, then letting go.
pub(super) fn drag_frames(from: Pos2, to: Pos2) -> Vec<Vec<egui::Event>> {
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let mut frames = vec![
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, true)],
    ];
    for step in 1..=4 {
        frames.push(vec![egui::Event::PointerMoved(
            from.lerp(to, step as f32 / 4.0),
        )]);
    }
    frames.push(vec![egui::Event::PointerMoved(to)]);
    frames.push(vec![button(to, false)]);
    frames.push(Vec::new());
    frames
}

/// Runs `frames` of the editor on `script`, saving the frame before
/// the button is let go as `name` when snapshots are asked for.
/// Runs the editor on `script` through `frames`; returns the texts the
/// last frame drew.
fn run_editor(script: &mut BlockScript, frames: Vec<Vec<egui::Event>>, name: &str) -> Vec<String> {
    let size = vec2(1500.0, 950.0);
    let editor = |script: &mut BlockScript, ui: &mut egui::Ui| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                show(
                    ui,
                    script,
                    BlockContext {
                        key: "drag test",
                        client: true,
                        highlighted: None,
                        objects: &[],
                    },
                );
            });
    };
    if let Some(folder) = std::env::var_os("AXCENITY_SNAPSHOTS") {
        // The same frames from the same start, stopping while the
        // block is still held.
        let mut replay = script.clone();
        let held = frames[..frames.len() - 2].to_vec();
        let path = std::path::PathBuf::from(folder).join(format!("{name}.png"));
        crate::snapshot::snapshot(&path, size, held, |ui| editor(&mut replay, ui));
    }
    let ctx = theme::test_context();
    // Half a second a frame, as the snapshot replays them, so animations
    // (a palette scrolling to a category) finish.
    let mut texts = Vec::new();
    for (index, events) in frames.into_iter().enumerate() {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            time: Some(index as f64 * 0.5),
            ..Default::default()
        };
        let output = crate::theme::run(&ctx, input, |ui| editor(script, ui));
        texts = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
    }
    texts
}

#[test]
fn blocks_drag_from_the_palette_and_snap_into_stacks() {
    let mut script = BlockScript::default();
    // "when game starts", the palette's first block (under the search
    // box), onto the canvas.
    run_editor(
        &mut script,
        drag_frames(pos2(150.0, 117.0 + SEARCH_HEIGHT), pos2(700.0, 400.0)),
        "drag-hat",
    );
    assert_eq!(script.blocks.len(), 1, "{script:?}");
    assert!(matches!(
        script.blocks[0].kind,
        BlockKind::WhenStarted { .. }
    ));
    let [x, y] = script.blocks[0]
        .position
        .expect("dropped where it was let go");

    // "wait 1 seconds" (the first Control block) to just under it: it
    // snaps into the event's stack. Clicking Control in the category
    // column first scrolls the palette to it.
    let hat_top_left = pos2(CATEGORY_WIDTH + PALETTE_WIDTH, TOOLBAR_HEIGHT) + vec2(x, y);
    let control = pos2(CATEGORY_WIDTH / 2.0, TOOLBAR_HEIGHT + 8.0 + 56.0 + 27.0);
    let click = |pressed| egui::Event::PointerButton {
        pos: control,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    // Where the palette draws it then (Inter's line height sets the rows).
    let wait_grab = pos2(110.0, 86.0 + SEARCH_HEIGHT);
    let wait_palette_top_left = pos2(88.0, 66.0 + SEARCH_HEIGHT);
    let target =
        hat_top_left + vec2(0.0, CAP + ROW_HEIGHT + 10.0) + (wait_grab - wait_palette_top_left);
    let mut frames = vec![
        vec![egui::Event::PointerMoved(control)],
        vec![click(true)],
        vec![click(false)],
        Vec::new(),
        Vec::new(),
    ];
    frames.extend(drag_frames(wait_grab, target));
    run_editor(&mut script, frames, "drag-wait");
    assert_eq!(script.blocks.len(), 1, "it joined the stack: {script:?}");
    let source = translate(&script).unwrap().source;
    assert_eq!(source, "task.spawn(function()\n    task.wait(1)\nend)\n");
}

#[test]
fn view_as_lua_opens_beside_the_canvas() {
    let mut script = sample_script(&[]);
    script.blocks.truncate(1);
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let button = pos2(1432.0, 20.0);
    let frames = vec![
        vec![egui::Event::PointerMoved(button)],
        vec![click(button, true)],
        vec![click(button, false)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];
    let before = script.clone();
    run_editor(&mut script, frames, "lua-panel");
    assert_eq!(script, before, "showing the Lua changes nothing");
}

#[test]
fn rounded_corners_stay_between_their_edges() {
    let square = [
        (pos2(0.0, 0.0), 4.0),
        (pos2(10.0, 0.0), 4.0),
        (pos2(10.0, 10.0), 4.0),
        (pos2(0.0, 10.0), 4.0),
    ];
    let points = rounded(&square, None, None);
    assert!(points.len() > 4);
    assert!(
        points
            .iter()
            .all(|point| { (0.0..=10.0).contains(&point.x) && (0.0..=10.0).contains(&point.y) })
    );
}

/// Every palette item the search finds for `search`, in a client script.
fn found(search: &str) -> Vec<PaletteItem> {
    let mut items = Vec::new();
    for category in Category::ALL {
        for item in palette(category, true, &[], &[]) {
            if matches_search(&search_text(&item, category, &[]), search) {
                items.push(item);
            }
        }
    }
    items
}

#[test]
fn the_palette_search_finds_blocks_by_their_words_and_their_lua() {
    let blocks = |search| {
        found(search)
            .into_iter()
            .filter_map(|item| match item {
                PaletteItem::Block(kind) => Some(*kind),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let everything = found("").len();
    assert!(everything > 100, "an empty search shows the whole palette");
    // By the words on the block, in any case and order.
    assert!(
        blocks("PLAY sound")
            .iter()
            .any(|kind| matches!(kind, BlockKind::PlaySound { .. }))
    );
    assert!(
        blocks("touched when")
            .iter()
            .any(|kind| matches!(kind, BlockKind::WhenTouched { .. }))
    );
    // By the Lua it becomes, for someone who knows the Roblox-style name.
    assert!(
        blocks("Destroy")
            .iter()
            .any(|kind| matches!(kind, BlockKind::Destroy { .. }))
    );
    assert!(
        found("Raycast")
            .iter()
            .any(|item| matches!(item, PaletteItem::Reporter(Expression::Raycast { .. })))
    );
    // A narrower search finds fewer, and nonsense finds nothing.
    assert!(found("sound").len() < everything);
    assert!(found("zzzz qqqq").is_empty());
}

#[test]
fn searching_the_palette_shows_the_matches_and_they_drag_out() {
    let mut script = BlockScript::default();
    let search_box = pos2(CATEGORY_WIDTH + 120.0, TOOLBAR_HEIGHT + SEARCH_HEIGHT / 2.0);
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let mut frames = vec![
        vec![egui::Event::PointerMoved(search_box)],
        vec![click(search_box, true)],
        vec![click(search_box, false)],
        vec![egui::Event::Text("wait until".to_owned())],
        Vec::new(),
    ];
    // The only match, "wait until", is first under its category's heading,
    // where "when game starts" is with no search.
    frames.extend(drag_frames(
        pos2(110.0, 86.0 + SEARCH_HEIGHT),
        pos2(700.0, 400.0),
    ));
    run_editor(&mut script, frames, "palette-search");
    assert_eq!(script.blocks.len(), 1, "{script:?}");
    let BlockKind::Unattached { body } = &script.blocks[0].kind else {
        panic!("a loose stack: {script:?}");
    };
    assert!(
        matches!(body[0].kind, BlockKind::WaitUntil { .. }),
        "{script:?}"
    );
}

#[test]
fn hovering_a_palette_block_says_the_lua_it_becomes() {
    let mut script = BlockScript::default();
    let hat = pos2(150.0, 117.0 + SEARCH_HEIGHT);
    // Held still long enough for the tooltip.
    let frames = vec![
        vec![egui::Event::PointerMoved(hat)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];
    let before = script.clone();
    let texts = run_editor(&mut script, frames, "palette-hover");
    assert_eq!(script, before, "hovering changes nothing");
    // The catalogue's card, then the Lua (cohesion pass CW4.2).
    for words in [
        "When script starts",
        "Runs the blocks inside once",
        "It becomes this Lua:",
    ] {
        assert!(
            texts.iter().any(|text| text.contains(words)),
            "{words}: {texts:?}"
        );
    }
    assert_eq!(
        palette::item_lua(&PaletteItem::Block(Box::new(BlockKind::WhenStarted {
            body: Vec::new()
        })))
        .as_deref(),
        Some("task.spawn(function()\nend)")
    );
}

#[test]
fn luau_blocks_fold_under_their_own_heading_but_stay_in_the_graph_menu() {
    let (own, luau) = palette_parts(Category::Variables, true, &[], &[]);
    let kinds = |items: &[PaletteItem]| {
        items
            .iter()
            .filter_map(|item| match item {
                PaletteItem::Block(kind) => Some(kind.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(kinds(&own).iter().all(|kind| matches!(
        **kind,
        BlockKind::SetVariable { .. }
            | BlockKind::ChangeVariable { .. }
            | BlockKind::SetAttribute { .. }
            | BlockKind::ListCreate { .. }
            | BlockKind::ListAdd { .. }
            | BlockKind::ListInsert { .. }
            | BlockKind::ListRemoveIndex { .. }
            | BlockKind::ListRemoveValue { .. }
            | BlockKind::ListClear { .. }
    )));
    assert!(
        kinds(&luau)
            .iter()
            .any(|kind| matches!(**kind, BlockKind::Local { .. }))
    );
    // The graph's node menu and searches still offer every block.
    assert_eq!(
        palette(Category::Variables, true, &[], &[]).len(),
        own.len() + luau.len()
    );
    assert!(!found("local").is_empty());

    // In the palette: Variables, then open "Lua blocks", then drag
    // "local name = 0" out.
    let mut script = BlockScript::default();
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let ctx = theme::test_context();
    let mut frame_idx = 0usize;
    let mut run = |events: Vec<egui::Event>| -> Vec<egui::epaint::ClippedShape> {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 1400.0))),
            events,
            time: Some(frame_idx as f64 * 0.5),
            ..Default::default()
        };
        frame_idx += 1;
        let mut out = crate::theme::run(&ctx, input, |ui| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    show(
                        ui,
                        &mut script,
                        BlockContext {
                            key: "palette-lua-blocks",
                            client: true,
                            highlighted: None,
                            objects: &[],
                        },
                    );
                });
        });
        out.textures_delta.clear();
        out.shapes
    };
    fn find_text(
        shapes: &[egui::epaint::ClippedShape],
        pred: impl Fn(&egui::epaint::TextShape, Rect) -> bool,
    ) -> Option<Pos2> {
        fn walk(
            shape: &egui::Shape,
            clip: Rect,
            pred: &impl Fn(&egui::epaint::TextShape, Rect) -> bool,
        ) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(t) if pred(t, clip) => Some(t.pos),
                egui::Shape::Vec(v) => v.iter().find_map(|s| walk(s, clip, pred)),
                _ => None,
            }
        }
        shapes
            .iter()
            .find_map(|s| walk(&s.shape, s.clip_rect, &pred))
    }
    let initial = run(Vec::new());
    let variables = find_text(&initial, |t, _| {
        t.galley.text() == "Variables" && t.pos.x < CATEGORY_WIDTH + 16.0
    })
    .expect("Variables category button should be rendered")
        + vec2(10.0, 4.0);
    for ev in [
        vec![egui::Event::PointerMoved(variables)],
        vec![click(variables, true)],
        vec![click(variables, false)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ] {
        run(ev);
    }
    let shapes = run(Vec::new());
    let vars_y = find_text(&shapes, |t, _| {
        t.galley.text() == "Variables" && t.pos.x > CATEGORY_WIDTH + 16.0
    })
    .expect("Variables heading should be rendered")
    .y;
    let heading = find_text(&shapes, |t, clip| {
        t.galley.text().contains("Lua blocks") && t.pos.y > vars_y && clip.contains(t.pos)
    })
    .expect("Variables Lua blocks heading should be rendered")
        + vec2(30.0, 8.0);
    for ev in [
        vec![egui::Event::PointerMoved(heading)],
        vec![click(heading, true)],
        vec![click(heading, false)],
        Vec::new(),
        Vec::new(),
    ] {
        run(ev);
    }
    for ev in drag_frames(heading + vec2(0.0, 36.0), pos2(700.0, 400.0)) {
        run(ev);
    }
    assert_eq!(script.blocks.len(), 1, "{script:?}");
    let BlockKind::Unattached { body } = &script.blocks[0].kind else {
        panic!("a loose stack: {script:?}");
    };
    assert!(
        matches!(body[0].kind, BlockKind::Local { .. }),
        "{script:?}"
    );
}

/// With Increase contrast (design system V18) every category's fill and
/// edge are dark enough for white labels; without it they're Axcenity's
/// own colours, which already give white labels 3:1 (usability brief §5).
#[test]
fn increase_contrast_darkens_block_colours_for_white_labels() {
    for dark in [true, false] {
        let quiet = theme::Palette::new(dark, theme::Accent::Blue);
        let high = quiet.with_contrast(true);
        for category in Category::ALL.into_iter().chain([Category::Lua]) {
            assert_eq!(category.fill(&quiet), category.base_fill());
            assert!(
                theme::contrast(egui::Color32::WHITE, category.fill(&quiet)) >= 3.0,
                "{category:?}"
            );
            for color in [category.fill(&high), category.edge(&high)] {
                assert!(
                    theme::contrast(egui::Color32::WHITE, color) >= 4.5,
                    "{category:?}"
                );
            }
        }
    }
}

/// Each category has its own colour, far enough from every other's to tell
/// apart, and its own icon (usability brief §5).
#[test]
fn every_category_has_a_colour_and_icon_of_its_own() {
    let all: Vec<Category> = Category::ALL.into_iter().chain([Category::Lua]).collect();
    for (index, a) in all.iter().enumerate() {
        for b in &all[index + 1..] {
            let (x, y) = (a.base_fill(), b.base_fill());
            let distance = [
                f32::from(x.r()) - f32::from(y.r()),
                f32::from(x.g()) - f32::from(y.g()),
                f32::from(x.b()) - f32::from(y.b()),
            ]
            .iter()
            .map(|d| d * d)
            .sum::<f32>()
            .sqrt();
            assert!(distance > 40.0, "{a:?} and {b:?} look alike: {distance}");
            assert_ne!(a.icon(), b.icon(), "{a:?} and {b:?}");
        }
    }
}

/// The tab under a block is one convex piece (egui fills only convex
/// shapes) that hangs `TAB_DEPTH` below the block, and its ends meet the
/// block's bottom edge.
#[test]
fn the_tab_is_a_convex_rounded_bump() {
    let at = pos2(100.0, 50.0);
    let points = painter::tab_points(at, 1.0);
    let (first, last) = (points[0], points[points.len() - 1]);
    assert_eq!(first, pos2(at.x + TAB_LEFT, at.y));
    assert!((last.x - (at.x + TAB_LEFT + TAB_WIDTH)).abs() < 1e-3 && (last.y - at.y).abs() < 1e-3);
    let deepest = points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!((deepest - (at.y + TAB_DEPTH)).abs() < 1e-3);
    // Convex: every turn along the outline (closed by the top edge) is the same way.
    let closed: Vec<Pos2> = points.clone();
    let count = closed.len();
    let turns: Vec<f32> = (0..count)
        .map(|i| {
            let (a, b, c) = (closed[i], closed[(i + 1) % count], closed[(i + 2) % count]);
            (b - a).x * (c - b).y - (b - a).y * (c - b).x
        })
        .filter(|turn| turn.abs() > 1e-4)
        .collect();
    assert!(
        turns.iter().all(|t| *t > 0.0) || turns.iter().all(|t| *t < 0.0),
        "{turns:?}"
    );
}

#[test]
fn the_toolbar_hint_never_runs_under_its_buttons() {
    let objects: Vec<ObjectInfo> = Vec::new();
    for width in [1500.0, 900.0, 640.0, 420.0] {
        let ctx = theme::test_context();
        let mut script = BlockScript::default();
        let mut output = None;
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 600.0))),
                ..Default::default()
            };
            output = Some(crate::theme::run(&ctx, input, |ui| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        show(
                            ui,
                            &mut script,
                            BlockContext {
                                key: "toolbar test",
                                client: false,
                                highlighted: None,
                                objects: &objects,
                            },
                        );
                    });
            }));
        }
        let texts: Vec<(String, Rect)> = output
            .unwrap()
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => {
                    // What shows of it.
                    let rect = text.visual_bounding_rect().intersect(clipped.clip_rect);
                    // Only what's wholly on the bar (the palette's icons
                    // start just under it).
                    (rect.is_positive()
                        && rect.bottom() <= TOOLBAR_HEIGHT + 0.5
                        && !text.galley.text().is_empty())
                    .then(|| (text.galley.text().to_owned(), rect))
                }
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|(text, _)| text.contains("Server blocks")),
            "{width}: the side chip is drawn: {texts:?}"
        );
        for (index, (a, a_rect)) in texts.iter().enumerate() {
            assert!(
                a_rect.right() <= width + 0.5,
                "{width}: `{a}` runs off the bar"
            );
            for (b, b_rect) in &texts[index + 1..] {
                assert!(
                    !a_rect.shrink(0.5).intersects(b_rect.shrink(0.5)),
                    "{width}: `{a}` {a_rect:?} overlaps `{b}` {b_rect:?}"
                );
            }
        }
        if width >= 900.0 {
            assert!(
                texts
                    .iter()
                    .any(|(text, _)| text.starts_with("Drag blocks")),
                "{width}: there's room for the hint: {texts:?}"
            );
        }
    }
}

/// Every block and reporter the palette offers is in the catalogue (CW4.1):
/// a title, what it does, and words for each of its inputs.
#[test]
fn every_palette_block_and_reporter_has_a_tooltip_card() {
    for client in [false, true] {
        for category in Category::ALL {
            for item in palette(category, client, &[], &["score".to_owned()]) {
                let Some(info) = palette::item_info(&item) else {
                    continue;
                };
                assert!(
                    !info.title.is_empty() && !info.description.is_empty(),
                    "{item:?}"
                );
                for (name, _) in &info.inputs {
                    assert!(
                        axcenity_blocks::catalogue::field_help(&name.replace(' ', "_")).is_some(),
                        "no words for the input `{name}` of {}",
                        info.title
                    );
                }
            }
        }
    }
}

#[test]
fn phase7_appearance_vfx_and_sound_pins_and_codegen() {
    use axcenity_blocks::{Block, BlockKind, BlockScript, Expression, ObjectRef, translate};

    let b = |id, kind| Block {
        id,
        kind,
        position: None,
        spaced: false,
        chained: false,
    };
    let num = |value| Expression::Number { value };
    let vec3 = |x, y, z| Expression::Vector3 { x, y, z };

    let script = BlockScript {
        blocks: vec![b(
            1,
            BlockKind::WhenStarted {
                body: vec![
                    b(
                        2,
                        BlockKind::SpawnDecal {
                            texture: Expression::Text {
                                value: "assets/textures/scorch.png".to_owned(),
                            },
                            position: vec3(0.0, 0.0, 0.0),
                            facing: vec3(0.0, 1.0, 0.0),
                            size: num(2.0),
                            seconds: num(5.0),
                        },
                    ),
                    b(
                        3,
                        BlockKind::Print {
                            value: Expression::BlockOutput {
                                block: 2,
                                output: 0,
                                name: "decal".to_owned(),
                            },
                        },
                    ),
                    b(
                        4,
                        BlockKind::SetClockTime {
                            time: num(20.0),
                            duration: Some(num(3.0)),
                        },
                    ),
                    b(
                        5,
                        BlockKind::SetAtmosphere {
                            property: "Haze".to_owned(),
                            value: num(4.0),
                            duration: Some(num(2.0)),
                        },
                    ),
                    b(
                        6,
                        BlockKind::SetGraphicsOption {
                            setting: "RenderScale".to_owned(),
                            choice: String::new(),
                            value: Some(num(0.75)),
                        },
                    ),
                    b(
                        7,
                        BlockKind::TweenProperty {
                            target: ObjectRef::path("Lamp"),
                            property: "Brightness".to_owned(),
                            value: num(4.0),
                            duration: num(1.5),
                            easing: "Quad".to_owned(),
                            wait_until_done: true,
                            completed: vec![],
                        },
                    ),
                    b(
                        8,
                        BlockKind::Print {
                            value: Expression::BlockOutput {
                                block: 7,
                                output: 0,
                                name: "tween".to_owned(),
                            },
                        },
                    ),
                    b(
                        9,
                        BlockKind::PlaySound {
                            target: ObjectRef::path("Chime"),
                            pitch: Some(num(1.2)),
                            wait_until_done: true,
                        },
                    ),
                    b(
                        10,
                        BlockKind::PlaySoundAtPosition {
                            target: ObjectRef::path("Boom"),
                            position: vec3(10.0, 2.0, -5.0),
                            volume: Some(num(0.8)),
                            pitch: Some(num(0.9)),
                        },
                    ),
                    b(
                        11,
                        BlockKind::FadeVolume {
                            target: ObjectRef::path("Music"),
                            volume: num(0.0),
                            duration: num(2.0),
                            wait_until_done: true,
                        },
                    ),
                    b(
                        12,
                        BlockKind::Print {
                            value: Expression::SoundProperty {
                                target: ObjectRef::path("Music"),
                                property: "Playing".to_owned(),
                            },
                        },
                    ),
                ],
            },
        )],
        graph: Default::default(),
        variables: vec![],
        tabs: false,
    };

    let lua = translate(&script).unwrap().source;
    assert!(
        lua.contains("local TweenService = game:GetService(\"TweenService\")"),
        "{lua}"
    );
    assert!(
        lua.contains("local Lighting = game:GetService(\"Lighting\")"),
        "{lua}"
    );
    assert!(
        lua.contains("math.abs(_dfacing.Unit.Y) > 0.99 and Vector3.zAxis or Vector3.yAxis"),
        "{lua}"
    );
    assert!(lua.contains("local decal = workspace:SpawnDecal("), "{lua}");
    assert!(
        lua.contains("TweenService:Create(Lighting, TweenInfo.new(3), { ClockTime = 20 }):Play()"),
        "{lua}"
    );
    assert!(
        lua.contains(
            "TweenService:Create(Lighting.Atmosphere, TweenInfo.new(2), { Haze = 4 }):Play()"
        ),
        "{lua}"
    );
    assert!(lua.contains("UserGameSettings.RenderScale = 0.75"), "{lua}");
    assert!(lua.contains("local tween = TweenService:Create("), "{lua}");
    assert!(lua.contains("tween.Completed:Wait()"), "{lua}");
    assert!(lua.contains("workspace.Chime.PlaybackSpeed = 1.2"), "{lua}");
    assert!(lua.contains("workspace.Chime.Ended:Wait()"), "{lua}");
    assert!(lua.contains("_s.Volume = 0.8"), "{lua}");
    assert!(lua.contains("_s.PlaybackSpeed = 0.9"), "{lua}");
    assert!(lua.contains("_tw.Completed:Wait()"), "{lua}");
    assert!(lua.contains("print(workspace.Music.Playing)"), "{lua}");
}

#[test]
fn phase8_operators_animation_and_tween_pins_and_codegen() {
    use axcenity_blocks::{MathFunction, ObjectRef};
    let block = |id: u64, kind: BlockKind| Block {
        id,
        kind,
        ..Block::new(BlockKind::StopScript)
    };

    let script = BlockScript {
        blocks: vec![
            block(
                1,
                BlockKind::WhenStarted {
                    body: vec![
                        // Scalar Lerp + Vector/Math/String/Color reporters
                        block(
                            2,
                            BlockKind::Print {
                                value: Expression::Lerp {
                                    from: Box::new(Expression::Number { value: 0.0 }),
                                    to: Box::new(Expression::Number { value: 10.0 }),
                                    alpha: Box::new(Expression::Number { value: 0.5 }),
                                },
                            },
                        ),
                        block(
                            3,
                            BlockKind::Print {
                                value: Expression::Function {
                                    function: MathFunction::Sin,
                                    value: Box::new(Expression::Number { value: 1.0 }),
                                },
                            },
                        ),
                        block(
                            4,
                            BlockKind::Print {
                                value: Expression::Clamp {
                                    value: Box::new(Expression::Distance {
                                        from: Box::new(Expression::MakeVector3 {
                                            x: Box::new(Expression::Number { value: 0.0 }),
                                            y: Box::new(Expression::Number { value: 0.0 }),
                                            z: Box::new(Expression::Number { value: 0.0 }),
                                        }),
                                        to: Box::new(Expression::Vector3 {
                                            x: 3.0,
                                            y: 4.0,
                                            z: 0.0,
                                        }),
                                    }),
                                    min: Box::new(Expression::Number { value: 0.0 }),
                                    max: Box::new(Expression::Number { value: 10.0 }),
                                },
                            },
                        ),
                        block(
                            5,
                            BlockKind::Print {
                                value: Expression::FormatNumber {
                                    value: Box::new(Expression::Number { value: 12.345 }),
                                    decimals: Box::new(Expression::Number { value: 2.0 }),
                                },
                            },
                        ),
                        block(
                            6,
                            BlockKind::Print {
                                value: Expression::StringContains {
                                    text: Box::new(Expression::Text {
                                        value: "hello".to_owned(),
                                    }),
                                    substring: Box::new(Expression::Text {
                                        value: "ell".to_owned(),
                                    }),
                                },
                            },
                        ),
                        block(
                            7,
                            BlockKind::Print {
                                value: Expression::MakeColorRgb {
                                    r: Box::new(Expression::Number { value: 255.0 }),
                                    g: Box::new(Expression::Number { value: 128.0 }),
                                    b: Box::new(Expression::Number { value: 0.0 }),
                                },
                            },
                        ),
                        // PlayAnimation with fade/speed/weight/wait + output
                        block(
                            8,
                            BlockKind::PlayAnimation {
                                animator: ObjectRef::path("Hero.Animator"),
                                animation: ObjectRef::path("Hero.Slash"),
                                fade_time: Some(Expression::Number { value: 0.2 }),
                                speed: Some(Expression::Number { value: 1.5 }),
                                weight: Some(Expression::Number { value: 1.0 }),
                                wait_until_done: true,
                            },
                        ),
                        // Tween with repeat/reverses/wait + Tween output referenced by CancelTween
                        block(
                            9,
                            BlockKind::Tween {
                                target: ObjectRef::path("Door"),
                                property: "Transparency".to_owned(),
                                value: Expression::Number { value: 1.0 },
                                seconds: Expression::Number { value: 0.5 },
                                style: "Quad".to_owned(),
                                direction: "Out".to_owned(),
                                repeat_count: Some(Expression::Number { value: 2.0 }),
                                reverses: true,
                                wait_until_done: false,
                                completed: vec![block(
                                    10,
                                    BlockKind::Print {
                                        value: Expression::Text {
                                            value: "done".to_owned(),
                                        },
                                    },
                                )],
                            },
                        ),
                        block(
                            11,
                            BlockKind::CancelTween {
                                tween: ObjectRef(Box::new(Expression::BlockOutput {
                                    block: 9,
                                    output: 0,
                                    name: "tween".to_owned(),
                                })),
                            },
                        ),
                        // SlideIn caches _RestPos and sets Visible = true
                        block(
                            12,
                            BlockKind::SlideIn {
                                target: ObjectRef::path("Hud.Menu"),
                                from: "Left".to_owned(),
                                seconds: Expression::Number { value: 0.3 },
                                wait_until_done: false,
                                completed: Vec::new(),
                            },
                        ),
                        // SlideOut hides on completion
                        block(
                            13,
                            BlockKind::SlideOut {
                                target: ObjectRef::path("Hud.Menu"),
                                to: "Left".to_owned(),
                                seconds: Expression::Number { value: 0.3 },
                                wait_until_done: false,
                                completed: Vec::new(),
                            },
                        ),
                    ],
                },
            ),
            // WhenAnimationEvent with Any Event + EventName context reporter
            block(
                20,
                BlockKind::WhenAnimationEvent {
                    animator: ObjectRef::path("Hero.Animator"),
                    event: "Any".to_owned(),
                    body: vec![block(
                        21,
                        BlockKind::Print {
                            value: Expression::AnimationEventName,
                        },
                    )],
                },
            ),
            // WhenStateChanged with NewState / PreviousState reporters
            block(
                22,
                BlockKind::WhenStateChanged {
                    animator: ObjectRef::path("Hero.Animator"),
                    body: vec![block(
                        23,
                        BlockKind::Print {
                            value: Expression::AnimatorNewState,
                        },
                    )],
                },
            ),
            // WhenAnimationFinished
            block(
                24,
                BlockKind::WhenAnimationFinished {
                    animator: ObjectRef::path("Hero.Animator"),
                    animation: ObjectRef::path("Hero.Slash"),
                    body: vec![block(
                        25,
                        BlockKind::Print {
                            value: Expression::Text {
                                value: "finished".to_owned(),
                            },
                        },
                    )],
                },
            ),
        ],
        graph: Default::default(),
        variables: vec![],
        tabs: false,
    };

    let lua = translate(&script).unwrap().source;
    assert!(lua.contains("print((0 + (10 - 0) * 0.5))"), "{lua}");
    assert!(lua.contains("print(math.sin(1))"), "{lua}");
    assert!(
        lua.contains("math.clamp((Vector3.new(0, 0, 0) - Vector3.new(3, 4, 0)).Magnitude, 0, 10)"),
        "{lua}"
    );
    assert!(lua.contains("string.format(\"%.2f\", 12.345)"), "{lua}");
    assert!(
        lua.contains("string.find(tostring(\"hello\"), tostring(\"ell\"), 1, true) ~= nil"),
        "{lua}"
    );
    assert!(lua.contains("Color3.fromRGB(255, 128, 0)"), "{lua}");
    assert!(lua.contains("_track:Play(0.2, 1, 1.5)"), "{lua}");
    assert!(lua.contains("_track.Stopped:Wait()"), "{lua}");
    assert!(lua.contains("Enum.EasingDirection.Out, 2, true)"), "{lua}");
    assert!(lua.contains("local tween = TweenService:Create("), "{lua}");
    assert!(lua.contains("tween:Cancel()"), "{lua}");
    assert!(lua.contains("GetAttribute(\"_RestPos\")"), "{lua}");
    assert!(lua.contains("workspace.Hud.Menu.Visible = true"), "{lua}");
    assert!(lua.contains("workspace.Hud.Menu.Visible = false"), "{lua}");
    assert!(
        lua.contains("AnimationEvent:Connect(function(name)\n    print(name)\nend)"),
        "{lua}"
    );
    assert!(
        lua.contains("StateChanged:Connect(function(state, previous)\n    print(state)\nend)"),
        "{lua}"
    );
    assert!(
        lua.contains("LoadAnimation(workspace.Hero.Slash).Stopped:Connect(function()"),
        "{lua}"
    );
}

#[test]
fn phase9_data_world_network_and_chat_pins_and_codegen() {
    use axcenity_blocks::ObjectRef;

    let block = |id: u64, kind: BlockKind| Block {
        id,
        kind,
        ..Block::new(BlockKind::StopScript)
    };

    let script = BlockScript {
        blocks: vec![
            block(
                10,
                BlockKind::WhenConnectionFails {
                    body: vec![block(
                        11,
                        BlockKind::Print {
                            value: Expression::ConnectionFailureReason,
                        },
                    )],
                },
            ),
            block(
                20,
                BlockKind::WhenLayerChanged {
                    data_layer: "Any".to_owned(),
                    body: vec![
                        block(
                            21,
                            BlockKind::Print {
                                value: Expression::ChangedLayerName,
                            },
                        ),
                        block(
                            22,
                            BlockKind::Print {
                                value: Expression::ChangedLayerActive,
                            },
                        ),
                    ],
                },
            ),
            block(
                30,
                BlockKind::WhenStreamedOut {
                    target: ObjectRef::path("Windmill"),
                    body: vec![block(
                        31,
                        BlockKind::Print {
                            value: Expression::Text {
                                value: "gone".to_owned(),
                            },
                        },
                    )],
                },
            ),
            block(
                40,
                BlockKind::WhenChatMessageReceived {
                    body: vec![
                        block(
                            41,
                            BlockKind::Print {
                                value: Expression::ChatMessageText,
                            },
                        ),
                        block(
                            42,
                            BlockKind::Print {
                                value: Expression::ChatMessageSender,
                            },
                        ),
                    ],
                },
            ),
            block(
                50,
                BlockKind::WhenStarted {
                    body: vec![
                        block(
                            51,
                            BlockKind::Print {
                                value: Expression::IsHost,
                            },
                        ),
                        block(
                            52,
                            BlockKind::Print {
                                value: Expression::IsConnected,
                            },
                        ),
                        block(
                            53,
                            BlockKind::Print {
                                value: Expression::SlotExists {
                                    slot: Box::new(Expression::Number { value: 1.0 }),
                                },
                            },
                        ),
                        block(
                            54,
                            BlockKind::DeleteSlot {
                                slot: Expression::Number { value: 1.0 },
                            },
                        ),
                        block(
                            55,
                            BlockKind::Print {
                                value: Expression::DataTableRowCount {
                                    table: "weapons".to_owned(),
                                },
                            },
                        ),
                        block(
                            56,
                            BlockKind::ForEachRow {
                                table: "weapons".to_owned(),
                                body: vec![block(
                                    57,
                                    BlockKind::Print {
                                        value: Expression::BlockOutput {
                                            block: 56,
                                            output: 0,
                                            name: "row_56".to_owned(),
                                        },
                                    },
                                )],
                            },
                        ),
                    ],
                },
            ),
        ],
        graph: Default::default(),
        variables: vec![],
        tabs: false,
    };

    let lua = translate(&script).unwrap().source;
    assert!(
        lua.contains("local DataLayerService = game:GetService(\"DataLayerService\")"),
        "{lua}"
    );
    assert!(
        lua.contains("local DataService = game:GetService(\"DataService\")"),
        "{lua}"
    );
    assert!(
        lua.contains("local NetworkService = game:GetService(\"NetworkService\")"),
        "{lua}"
    );
    assert!(
        lua.contains("local SaveService = game:GetService(\"SaveService\")"),
        "{lua}"
    );
    assert!(
        lua.contains("local TextChatService = game:GetService(\"TextChatService\")"),
        "{lua}"
    );
    assert!(
        lua.contains(
            "NetworkService.ConnectionFailed:Connect(function(reason)\n    print(reason)\nend)"
        ),
        "{lua}"
    );
    assert!(
        lua.contains("DataLayerService.LayerChanged:Connect(function(layerName, active)\n    print(layerName)\n    print(active)\nend)"),
        "{lua}"
    );
    assert!(
        lua.contains("workspace.Windmill.StreamedOut:Connect(function()"),
        "{lua}"
    );
    assert!(
        lua.contains("TextChatService.MessageReceived:Connect(function(message)\n    print(message.Text)\n    print(message.Player)\nend)"),
        "{lua}"
    );
    assert!(lua.contains("print(NetworkService.IsHost)"), "{lua}");
    assert!(lua.contains("print(NetworkService.IsConnected)"), "{lua}");
    assert!(lua.contains("print(SaveService:HasSlot(1))"), "{lua}");
    assert!(lua.contains("SaveService:DeleteSlot(1)"), "{lua}");
    assert!(
        lua.contains("print(DataService:GetTable(\"weapons\"):Count())"),
        "{lua}"
    );
    assert!(
        lua.contains("for i_56, row_56 in DataService:GetTable(\"weapons\"):GetRows() do\n        print(row_56)\n    end"),
        "{lua}"
    );
}

#[test]
fn palette_splitter_drag_resizes_sidebar() {
    let context = egui::Context::default();
    theme::install(&context);
    let mut script = BlockScript::default();
    let splitter_x = CATEGORY_WIDTH + PALETTE_WIDTH - PALETTE_SPLITTER_WIDTH * 0.5;
    let start = pos2(splitter_x, TOOLBAR_HEIGHT + 200.0);
    let end = pos2(splitter_x + 120.0, TOOLBAR_HEIGHT + 200.0);
    let frames = drag_frames(start, end);
    let size = vec2(1500.0, 900.0);
    for events in frames {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        };
        let mut out = context.run_ui(input, |ui| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    show(
                        ui,
                        &mut script,
                        BlockContext {
                            key: "splitter-test",
                            client: true,
                            highlighted: None,
                            objects: &[],
                        },
                    );
                });
        });
        out.textures_delta.clear();
    }
    let width = context
        .data_mut(|data| data.get_persisted::<f32>(egui::Id::new("block_editor_palette_width")))
        .expect("dragging the splitter should persist the resized palette width");
    assert!(
        width > PALETTE_WIDTH + 80.0,
        "expected palette width to grow after dragging splitter right, got {width}"
    );
}
