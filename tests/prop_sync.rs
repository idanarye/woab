use gtk::prelude::*;

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct TestWidgets {
    #[prop_sync(set, get)]
    #[widget(nested)]
    group1: WidgetsGroup1,
    #[prop_sync(set, get)]
    #[widget(nested)]
    group2: WidgetsGroup2,
}

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct WidgetsGroup1 {
    #[prop_sync(set, get)]
    text_entry: gtk::Entry,
    #[prop_sync("value": f64, set, get)]
    spin_button: gtk::SpinButton,
}

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct WidgetsGroup2 {
    #[prop_sync("active": bool, set, get)]
    check_button: gtk::CheckButton,
}

#[test]
fn test_prop_sync() -> anyhow::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("tests/various_widgets.glade")?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;

    let widgets: TestWidgets = factory.instantiate().widgets()?;

    widgets.group1.text_entry.set_text("one");
    widgets.group1.spin_button.set_value(2.0);
    widgets.group2.check_button.set_active(false);

    let TestWidgetsPropGetter {
        group1: WidgetsGroup1PropGetter { text_entry, spin_button },
        group2: WidgetsGroup2PropGetter { check_button },
    } = widgets.get_props();

    assert_eq!(text_entry, "one");
    assert_eq!(spin_button as i64, 2);
    assert!(!check_button);

    widgets.set_props(&TestWidgetsPropSetter {
        group1: WidgetsGroup1PropSetter {
            text_entry: "three",
            spin_button: 4.0,
        },
        group2: WidgetsGroup2PropSetter { check_button: true },
    });

    assert_eq!(widgets.group1.text_entry.get_text(), "three");
    assert_eq!(widgets.group1.spin_button.get_value_as_int(), 4);
    assert!(widgets.group2.check_button.get_active());

    Ok(())
}
