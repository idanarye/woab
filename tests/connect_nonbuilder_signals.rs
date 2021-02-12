use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use actix::prelude::*;
use gtk::prelude::*;
use gio::prelude::*;

#[macro_use]
mod util;

#[derive(Debug, PartialEq, woab::BuilderSignal)]
enum TestSignal {
    Action1,
    Action2,
    BlockAction(gio::SimpleAction, #[signal(variant)] String),
    UnblockAction(gio::SimpleAction, #[signal(variant)] String),
    DisconnectAction(gio::SimpleAction, #[signal(variant)] String),
}

struct TestActor {
    action_group: gio::SimpleActionGroup,
    output: Rc<RefCell<Vec<&'static str>>>,
    actions: HashMap<&'static str, (gio::SimpleAction, glib::signal::SignalHandlerId)>,
}

impl actix::Actor for TestActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let router = TestSignal::connector().route_to::<TestActor>(ctx);

        for (action_name, signal_name) in &[
            ("action1", "Action1"),
            ("action2", "Action2"),
        ] {
            let action = gio::SimpleAction::new(action_name, None);
            self.action_group.add_action(&action);
            self.actions.insert(action_name, (
                    action.clone(),
                    action.connect_local("activate", false, router.handler(signal_name).unwrap()).unwrap(),
            ));
        }
        for (action_name, signal_name) in &[
            ("block", "BlockAction"),
            ("unblock", "UnblockAction"),
            ("disconnect", "DisconnectAction"),
        ] {
            let action = gio::SimpleAction::new(action_name, Some(&*String::static_variant_type()));
            self.action_group.add_action(&action);
            action.connect_local("activate", false, router.handler(signal_name).unwrap()).unwrap();
        }

        self.output.borrow_mut().push("init");
    }
}

impl actix::StreamHandler<TestSignal> for TestActor {
    fn handle(&mut self, signal: TestSignal, _ctx: &mut Self::Context) {
        match signal {
            TestSignal::Action1 => {
                self.output.borrow_mut().push("action1");
            },
            TestSignal::Action2 => {
                self.output.borrow_mut().push("action2");
            },
            TestSignal::BlockAction(_, action) => {
                let (action, signal) = &self.actions[action.as_str()];
                action.block_signal(signal);
                self.output.borrow_mut().push("block");
            }
            TestSignal::UnblockAction(_, action) => {
                let (action, signal) = &self.actions[action.as_str()];
                action.unblock_signal(signal);
                self.output.borrow_mut().push("unblock");
            }
            TestSignal::DisconnectAction(_, action) => {
                let (action, signal) = self.actions.remove(&*action).unwrap();
                action.disconnect(signal);
                self.output.borrow_mut().push("disconnect");
            }
        }
    }
}

#[test]
fn test_connect_nonbuilder_signals() -> anyhow::Result<()> {
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop("test")?;

    let output = Rc::new(RefCell::new(Vec::new()));

    let action_group = gio::SimpleActionGroup::new();
    TestActor {
        action_group: action_group.clone(),
        output: output.clone(),
        actions: Default::default(),
    }.start();

    wait_for!(*output.borrow() == ["init"])?;
    action_group.activate_action("action1", None);
    wait_for!(*output.borrow() == ["init", "action1"])?;
    action_group.activate_action("action2", None);
    wait_for!(*output.borrow() == ["init", "action1", "action2"])?;
    action_group.activate_action("block", Some(&"action1".to_variant()));
    wait_for!(*output.borrow() == ["init", "action1", "action2", "block"])?;

    // We send both action1 and action2, but action1 is blocked
    action_group.activate_action("action1", None);
    action_group.activate_action("action2", None);
    wait_for!(*output.borrow() == ["init", "action1", "action2", "block", "action2"])?;

    action_group.activate_action("unblock", Some(&"action1".to_variant()));
    wait_for!(*output.borrow() == ["init", "action1", "action2", "block", "action2", "unblock"])?;
    action_group.activate_action("disconnect", Some(&"action2".to_variant()));
    wait_for!(*output.borrow() == ["init", "action1", "action2", "block", "action2", "unblock", "disconnect"])?;

    // We send both action2 and action1, but action2 is disconnected
    action_group.activate_action("action2", None);
    action_group.activate_action("action1", None);
    wait_for!(*output.borrow() == ["init", "action1", "action2", "block", "action2", "unblock", "disconnect", "action1"])?;

    Ok(())
}
