use std::cell::RefCell;
use std::rc::Rc;

// use gtk::prelude::*;
use gio::prelude::*;

#[macro_use]
mod util;

#[test]
fn test_waking() -> anyhow::Result<()> {
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;

    let output = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    woab::block_on(async {
        let output = output.clone();
        actix::spawn(async move {
            let action1 = gio::SimpleAction::new("action1", None);
            output.borrow_mut().push("before action1");
            let action_response = woab::wake_from(|tx| {
                let output = output.clone();
                action1.connect_activate(move |_, _| {
                    let _ = tx.try_send(12);
                    output.borrow_mut().push("inside action1");
                });
                action1.activate(None);
            })
            .await
            .unwrap();
            assert_eq!(action_response, 12);
            output.borrow_mut().push("after action1");

            let action2 = gio::SimpleAction::new("action2", None);
            let action_response = woab::wake_from_signal(&action2, |tx| {
                let output = output.clone();
                let signal_handler_id = action2.connect_activate(move |_, _| {
                    let _ = tx.try_send(13);
                    output.borrow_mut().push("inside action2 1");
                });
                action2.activate(None);
                signal_handler_id
            })
            .await
            .unwrap();
            assert_eq!(action_response, 13);
            let action_response = woab::wake_from_signal(&action2, |tx| {
                let output = output.clone();
                let signal_handler_id = action2.connect_activate(move |_, _| {
                    let _ = tx.try_send(14);
                    output.borrow_mut().push("inside action2 2");
                });
                action2.activate(None);
                signal_handler_id
            })
            .await
            .unwrap();
            assert_eq!(action_response, 14);
            output.borrow_mut().push("after action2");
        });
    });

    wait_for!(
        *output.borrow()
            == [
                "before action1",
                "inside action1",
                "after action1",
                "inside action2 1",
                "inside action2 2",
                "after action2",
            ]
    )?;

    Ok(())
}
