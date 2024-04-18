use std::cell::RefCell;
use std::rc::Rc;

use actix::prelude::*;
use gio::prelude::*;
use hashbrown::HashMap;

#[macro_use]
mod util;

struct TestActor {
    action_group: gio::SimpleActionGroup,
    output: Rc<RefCell<Vec<&'static str>>>,
    actions: HashMap<&'static str, (gio::SimpleAction, glib::signal::SignalHandlerId)>,
}

impl actix::Actor for TestActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        for action_name in &["action1", "action2"] {
            let action = gio::SimpleAction::new(action_name, None);
            self.action_group.add_action(&action);
            self.actions.insert(
                action_name,
                (action.clone(), woab::route_action(&action, ctx.address()).unwrap()),
            );
        }
        for action_name in &["block", "unblock", "disconnect"] {
            let action = gio::SimpleAction::new(action_name, Some(&*String::static_variant_type()));
            self.action_group.add_action(&action);
            woab::route_action(&action, ctx.address()).unwrap();
        }

        self.output.borrow_mut().push("init");
    }
}

impl actix::Handler<woab::Signal> for TestActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "action1" => {
                self.output.borrow_mut().push("action1");
                None
            }
            "action2" => {
                self.output.borrow_mut().push("action2");
                None
            }
            "block" => {
                let action = msg.param::<glib::Variant>(1)?;
                let action = action.str().unwrap();
                let (action, signal) = &self.actions[action];
                action.block_signal(signal);
                self.output.borrow_mut().push("block");
                None
            }
            "unblock" => {
                let action = msg.param::<glib::Variant>(1)?;
                let action = action.str().unwrap();
                let (action, signal) = &self.actions[action];
                action.unblock_signal(signal);
                self.output.borrow_mut().push("unblock");
                None
            }
            "disconnect" => {
                let action = msg.param::<glib::Variant>(1)?;
                let action = action.str().unwrap();
                let (action, signal) = self.actions.remove(action).unwrap();
                action.disconnect(signal);
                self.output.borrow_mut().push("disconnect");
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

#[test]
fn test_connect_nonbuilder_signals() -> anyhow::Result<()> {
    util::test_main(async {
        let output = Rc::new(RefCell::new(Vec::new()));

        let action_group = gio::SimpleActionGroup::new();
        TestActor {
            action_group: action_group.clone(),
            output: output.clone(),
            actions: Default::default(),
        }
        .start();

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
        wait_for!(
            *output.borrow()
                == [
                    "init",
                    "action1",
                    "action2",
                    "block",
                    "action2",
                    "unblock",
                    "disconnect",
                    "action1"
                ]
        )?;
        Ok(())
    })
}
