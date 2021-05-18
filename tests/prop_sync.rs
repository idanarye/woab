use gtk::prelude::*;

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct TestWidgets {
    // win_test: gtk::ApplicationWindow,
    #[prop_sync(set, get)]
    text_entry: gtk::Entry,
    #[prop_sync("value": f64, set, get)]
    spin_button: gtk::SpinButton,
    #[prop_sync("active": bool, set, get)]
    check_button: gtk::CheckButton,
}

#[test]
fn test_prop_sync() -> anyhow::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("tests/various_widgets.glade")?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;

    let widgets: TestWidgets = factory.instantiate().widgets()?;

    widgets.text_entry.set_text("one");
    widgets.spin_button.set_value(2.0);
    widgets.check_button.set_active(false);

    let TestWidgetsPropGetter {
        text_entry,
        spin_button,
        check_button,
    } = widgets.get_props();

    assert_eq!(text_entry, "one");
    assert_eq!(spin_button, 2.0);
    assert!(!check_button);

    widgets.set_props(&TestWidgetsPropSetter {
        text_entry: "three",
        spin_button: 4.0,
        check_button: true,
    });

    assert_eq!(widgets.text_entry.get_text(), "three");
    assert_eq!(widgets.spin_button.get_value(), 4.0);
    assert!(widgets.check_button.get_active());

    Ok(())
}
