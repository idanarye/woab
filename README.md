[![Build Status](https://github.com/idanarye/woab/workflows/CI/badge.svg)](https://github.com/idanarye/woab/actions)
[![Latest Version](https://img.shields.io/crates/v/woab.svg)](https://crates.io/crates/woab)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://idanarye.github.io/woab/)

**IMPORTANT!!!** The signal handling API is going to go through a big, breaking overhaul. See https://github.com/idanarye/woab/issues/15.

# WoAB

WoAB (Widgets on Actors Bridge) is a library for combining the widgets toolkit
[GTK](https://gtk-rs.org/) with the actors framework
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

* GTK requires some signals to return a boolean value - `true` to "inhibit" and
  not let the signal pass up the inheritance to other handlers, and `false` to
  let it. WoAB cannot automatically detect which signals need it and which not,
  and will return `None` by default.  To set the value, use `#[signal(inhibit =
  ...)]` on the signal variant in the `BuilderSignal` derive macro or use the
  `inhibit()` method of `BuilderConnector`.
* If multiple tagged signals are streamed to the same actor - which is the
  typical use case for tagged signals - `StreamHandler::finished` should be
  overridden to avoid stopping the actor when one instance of the widgets is
  removed!!!
* If you connect signals via a builder connector, they will only be connected
  once the connector is dropped. If you need the signals connected before the
  connector is naturally dropped (e.g. - if you start `gtk::main()` in the same
  scope) use the `finish()` method of the builder connector.

## License

Licensed under MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT))
