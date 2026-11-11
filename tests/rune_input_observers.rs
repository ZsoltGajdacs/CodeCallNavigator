//! Native input observers must hear interruptions without consuming normal input.
use rune::{App, AppSpec, Cx, El, Env, PointerButton, Runtime, Tx, el};

#[derive(Default)]
struct State {
    observed: usize,
    pressed: usize,
}

fn view(_: &State, _: &mut Cx<State>) -> El {
    el().w_full().h_full().child(
        el().key("button")
            .size(40.0, 40.0)
            .on_press(|s: &mut State| s.pressed += 1),
    )
}

fn observe(s: &mut State, tx: &mut Tx) {
    s.observed += 1;
    tx.live_only();
}

#[test]
fn pointer_observer_hears_blank_space_and_keeps_normal_dispatch() {
    let mut rt = Runtime::new(State::default(), view, (200.0, 100.0), Env::default());
    rt.on_pointer_activity = Some(observe);
    rt.frame(0.0);
    rt.pointer_move(100.0, 80.0);
    rt.pointer_down(100.0, 80.0);
    rt.pointer_up(100.0, 80.0);
    rt.secondary_down(100.0, 80.0);
    rt.secondary_up(100.0, 80.0);
    rt.button_down(100.0, 80.0, PointerButton::Middle);
    rt.button_up(100.0, 80.0, PointerButton::Middle);
    rt.wheel(100.0, 80.0, 10.0);
    assert_eq!(rt.state.observed, 8);
    rt.pointer_down(20.0, 20.0);
    rt.pointer_up(20.0, 20.0);
    rt.frame(0.0);
    assert_eq!(rt.state.pressed, 1);
}

#[test]
fn app_passes_pointer_observer_to_headless_runtime() {
    let app = App::new(AppSpec::new(
        "Pointer observer",
        (200.0, 100.0),
        State::default,
        view,
    ))
    .on_pointer_activity(observe);
    let mut rt = app.headless_runtime((200.0, 100.0), Env::default());
    rt.pointer_down(100.0, 80.0);
    assert_eq!(rt.state.observed, 1);
}

#[test]
fn key_observer_hears_text_consumed_by_a_focused_editor() {
    #[derive(Default)]
    struct Form {
        edit: rune::TextEdit,
        observed: Vec<rune::Key>,
        app_keys: usize,
    }
    fn form(s: &Form, _: &mut Cx<Form>) -> El {
        rune::text_input(&s.edit, rune::TextStyle::new(14.0, 400.0))
            .key("edit")
            .size(160.0, 30.0)
            .on_edit(|s: &mut Form| &mut s.edit)
    }
    let mut rt = Runtime::new(Form::default(), form, (200.0, 100.0), Env::default());
    rt.on_key_activity = Some(|s, ev, tx| {
        s.observed.push(ev.key.clone());
        tx.live_only();
    });
    rt.on_key = Some(|s, _, _| {
        s.app_keys += 1;
        false
    });
    rt.frame(0.0);
    rt.pointer_down(20.0, 15.0);
    rt.pointer_up(20.0, 15.0);
    rt.frame(0.0);
    rt.key(rune::KeyEvent {
        key: rune::Key::Text("X".into()),
        mods: Default::default(),
    });
    rt.frame(0.0);
    assert_eq!(rt.state.edit.text, "X");
    assert_eq!(rt.state.observed, vec![rune::Key::Text("X".into())]);
    assert_eq!(rt.state.app_keys, 0);
}

#[test]
fn modifier_observer_reports_each_logical_transition_once() {
    let mut rt = Runtime::new(
        Vec::<rune::Key>::new(),
        |_, _| el(),
        (200.0, 100.0),
        Env::default(),
    );
    rt.set_modifier_keys(true);
    rt.on_key_activity = Some(|s, ev, tx| {
        s.push(ev.key.clone());
        tx.live_only();
    });
    rt.frame(0.0);
    for pressed in [true, false] {
        rt.key(rune::KeyEvent {
            key: rune::Key::Modifier {
                modifier: rune::Modifier::ShiftLeft,
                pressed,
            },
            mods: rune::Mods {
                shift: pressed,
                ..Default::default()
            },
        });
        rt.frame(0.0);
    }
    assert_eq!(rt.state.len(), 2);
}
