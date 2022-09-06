use gtk::prelude::*;

#[macro_use]
mod util;

#[derive(woab::WidgetsFromBuilder)]
pub struct FlatWidgets {
    text1: gtk::Entry,
    text2: gtk::Entry,
    text3: gtk::Entry,
    text4: gtk::Entry,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct GroupedWidgets {
    #[widget(nested)]
    group_a: GroupA,
    #[widget(nested)]
    group_b: GroupB,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct GroupA {
    text1: gtk::Entry,
    text2: gtk::Entry,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct GroupB {
    text3: gtk::Entry,
    text4: gtk::Entry,
}

#[test]
fn test_recusive_widgets_from_builder() -> anyhow::Result<()> {
    let factory = woab::BuilderFactory::from(include_str!("four_texts.glade").to_owned());
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop();
    let bld = factory.instantiate();

    let flat_widgets: FlatWidgets = bld.widgets()?;
    let grouped_widgets: GroupedWidgets = bld.widgets()?;

    assert!(grouped_widgets.group_a.text1.text().is_empty());
    assert!(grouped_widgets.group_a.text2.text().is_empty());
    assert!(grouped_widgets.group_b.text3.text().is_empty());
    assert!(grouped_widgets.group_b.text4.text().is_empty());

    flat_widgets.text1.set_text("Text 1");
    flat_widgets.text2.set_text("Text 2");
    flat_widgets.text3.set_text("Text 3");
    flat_widgets.text4.set_text("Text 4");

    wait_for!(flat_widgets.text1.text() == "Text 1")?;
    wait_for!(flat_widgets.text2.text() == "Text 2")?;
    wait_for!(flat_widgets.text3.text() == "Text 3")?;
    wait_for!(flat_widgets.text4.text() == "Text 4")?;

    assert!(grouped_widgets.group_a.text1.text() == "Text 1");
    assert!(grouped_widgets.group_a.text2.text() == "Text 2");
    assert!(grouped_widgets.group_b.text3.text() == "Text 3");
    assert!(grouped_widgets.group_b.text4.text() == "Text 4");

    woab::close_actix_runtime()??;
    Ok(())
}
