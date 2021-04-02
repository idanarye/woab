# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `woab::block_on`, for running the Tokio runtime and Actix system WoAB is
  using.
- `woab::Signal` - a dynamic signal type to replace `BuilderSignal`.
  - Unlike `BuilderSignal`, `woab::Signal` handling is done while the GTK
    signal is running, and it is an Actix message that can have a result - the
    inhibitness.
- `BuilderConnector::connect_to` and `BuilderConnector::connect_with` to
  connect the builder signals to actors using `woab::Signal`.
- `woab::route_signal` to route individual signals directly from the GTK
  object, without a builder.
- `woab::NamespacedSignalRouter` for routing signals from the same builder to
  different actors.
- `BuilderConnector::with_object`.

### Changed
- [**BREAKING**] Updated Actix to 0.11 and Tokio to 1.14. Consequences:
  - Actors can no longer just be started from outide Tokio/Actix. Instead, they
    must be started in a future (`async` block) passed the new `woab::block_on`
    function.
  - `woab::run_actix_inside_gtk_event_loop` no longer accepts a name.

### Removed
- [**BREAKING**] Removed everything related to `BuilderSignal` - the derive
  macro, the trait, and all the builder connector methods and helper structs.
  Use `woab::Signal` instead.

## 0.2.1 - 2021-03-18
### Fixed
- Fix the version of the macro crate.

## 0.2.0 - 2021-03-01
### Added
- `#[signal(inhibit = true|false)]` attribute for
  `#[derive(woab::BuilderSignal)]` for statically setting the return type of
  the signal.
- `inhibit` method for choosing the return type of signals based on signal
  parameters.
- `#[widget(name = "...")]` for overriding a widget's name when connecting widgets.
- `#[siganl(event)]` for parsing signal arguments that are `gdk::Event` to
  their specialized event type.
- `#[siganl(variant)]` for parsing signal arguments that are `glib::Variant` to
  their concrete type.

### Changed
- [**BREAKING**] `Factories` no longer defines the types of actor, signal and
  widgets. These are now defined when the widgets is created and connected to
  the actor.
- [**BREAKING**] Builder utilization syntax drastically changed:
  - `instantiate` is the new "entry point" (instead of `build`) for starting
    the widgets&actor creation.
  - Signals are connected explicitly, and can be connected from multiple
    `BuilderSignal` enums.
  - Widgets are only created on demand, and can be created multiple times with
    different types (that `impl TryFrom<gtk::Builder>`).
  - The actor is created with a `start` method - which accepts the actor
    directly (not a closure that creates it) or with `create`/`try_create`
    which accepts a closure that accepts an enhanced context (unlike Actix
    context and widgets. The widgets can be created from the enhanced context)
- Signals are now routed to actors through unbounded Tokio channels.

## 0.1.0 - 2020-09-02
### Added
- `woab::run_actix_inside_gtk_event_loop()` for allowing running Actix and GTK in the same thread.
- Glade XML dissection facilities.
- `woab::Factory` for creating widgets and/or actors and connecting them.
- Custom derives `WidgetsFromBuilder`, `BuilderSignal`, `Factories`, `Removable`.
