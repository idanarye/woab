[![Build Status](https://github.com/idanarye/woab/workflows/WoAB-CI/badge.svg)](https://github.com/idanarye/woab/actions)
[![Latest Version](https://img.shields.io/crates/v/woab.svg)](https://crates.io/crates/woab)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://idanarye.github.io/woab/)

# WoAB

WoAB (Widgets on Actors Bridge) is a library for combining the widgets toolkit [GTK](https://gtk-rs.org/) with the actors framework [Actix](https://actix.rs/). It helps with:

* Running the actors inside the GTK thread, allowing message handlers to interact with the widgets directly.
* Routing GTK signals through the asynchronous runtime, so that the code handling them can proceed naturally to interact with the actors.
* Mapping widgets and signals from [Glade](https://glade.gnome.org/) XML files to user types.

Refer to [the docs](https://idanarye.github.io/woab/) for more explanation on how to use WoAB, and to [the examples](https://github.com/idanarye/woab/tree/master/examples) for a short demonstration.

## License

Licensed under MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT))
