use gtk4::prelude::*;

#[macro_use]
mod util;

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct TestWidgets {
    #[prop_sync(set, get)]
    #[widget(nested)]
    group1: WidgetsGroup1,
    #[prop_sync(set, get)]
    #[widget(nested)]
    group2: WidgetsGroup2,
    #[prop_sync(set)]
    label: gtk4::Label,
}

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct WidgetsGroup1 {
    #[prop_sync(set, get)]
    text_entry: gtk4::Entry,
    #[prop_sync("value" as f64, set, get)]
    spin_button: gtk4::SpinButton,
}

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct WidgetsGroup2 {
    #[prop_sync(set, get)]
    check_button: gtk4::CheckButton,
}

#[test]
fn test_prop_sync() -> anyhow::Result<()> {
    util::test_main(async {
        let factory = woab::BuilderFactory::from(std::fs::read_to_string("tests/various_widgets.ui")?);

        let widgets: TestWidgets = factory.instantiate_without_routing_signals().widgets()?;

        widgets.group1.text_entry.set_text("one");
        widgets.group1.spin_button.set_value(2.0);
        widgets.group2.check_button.set_active(false);
        widgets.label.set_text("three");

        let TestWidgetsPropGetter {
            group1: WidgetsGroup1PropGetter { text_entry, spin_button },
            group2: WidgetsGroup2PropGetter { check_button },
        } = widgets.get_props();

        assert_eq!(text_entry, "one");
        assert_eq!(spin_button as i64, 2);
        assert!(!check_button);

        widgets.set_props(&TestWidgetsPropSetter {
            group1: WidgetsGroup1PropSetter {
                text_entry: "four",
                spin_button: 5.0,
            },
            group2: WidgetsGroup2PropSetter { check_button: true },
            label: "six",
        });

        assert_eq!(widgets.group1.text_entry.text(), "four");
        assert_eq!(widgets.group1.spin_button.value_as_int(), 5);
        assert!(widgets.group2.check_button.is_active());
        assert_eq!(widgets.label.text(), "six");
        Ok(())
    })
}
