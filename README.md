[![Build Status](https://github.com/idanarye/woab/workflows/CI/badge.svg)](https://github.com/idanarye/woab/actions)
[![Latest Version](https://img.shields.io/crates/v/woab.svg)](https://crates.io/crates/woab)
[![Rust Documentation - Latest Version](https://img.shields.io/badge/docs-released-blue.svg)](https://docs.rs/woab)
[![Rust Documentation - Nightly](https://img.shields.io/badge/docs-nightly-purple.svg)](https://idanarye.github.io/woab/)

# WoAB

WoAB (Widgets on Actors Bridge) is a GUI microframework for combining the
widgets toolkit [GTK](https://gtk-rs.org/) with the actors framework
[Actix](https://actix.rs/). It helps with:

* Running the actors inside the GTK thread, allowing message handlers to
  interact with the widgets directly.
* Routing GTK signals through the asynchronous runtime, so that the code
  handling them can proceed naturally to interact with the actors.
* Mapping widgets and signals from [Glade](https://glade.gnome.org/) XML files
  to user types.

Refer to [the docs](https://idanarye.github.io/woab/) for more explanation on
how to use WoAB, and to [the
examples](https://github.com/idanarye/woab/tree/master/examples) for a short
demonstration.

## Pitfalls

* When starting Actix actors from outside Tokio/Actix, `woab::block_on` must be
  used. This is a limitation of Actix that needs to be respected.
* Some GTK actions (like removing a widget) can fire signals synchronously. If
  these signals are registered as builder signals, WoAB will not be able to
  route them and panic because it will happen while the Actix runtime is
  occupied. To work around this, use `woab::spawn_outside`.
* `dialog.run()` must not be used - use `woab::run_dialog` instead.

## License

Licensed under MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT))
