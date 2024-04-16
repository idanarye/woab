# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- [**BREAKING**] Upgrade to GTK4
- Change the syntax of the `PropSync` derive's attribute from
  ```rust
  `#[prop_sync("value": Type, ...)]
  ```
  to
  ```rust
  #[prop_sync("value" as Type, ...)]
  ```
- `Removable` derive's attribute now needs to specify, in addition to the field
  that needs to be removed, the type of the container. The syntax is:
  ```rust
  #[removable(self.path.to.widget.field in gtk4::TypeOfContainer)]
  ```

### Removed
- `run_dialog`. It is no longer required, since dialogs fully support async now.
- `event_param`. It seems like `gdk4::Event` is a low-level implementation
  detail, unlike GTK3's `gdk::Event` which is used as the details parameter of
  some signals.

## 0.8.0 - 2023-08-19
### Changed
- [**BREAKING**] Upgraded gtk-rs to version 0.18. This is a breaking change
  because gtk-rs' API had some changes in this release:
  - Instead of `gtk::Inhibit`, use `glib::Propagation`. `Inhibit(false)` should
    become `Propagation::Stop` and `Inhibit(true)` should become
    `Propagation::Proceed`.

## 0.7.0 - 2022-09-06
### Changed
- [**BREAKING**] Upgraded gtk-rs to version 0.15. This is a breaking change
  because gtk-rs' API had some changes in this release.
- Updated Actix version to 0.13.
- **BREAKING** `woab::close_actix_runtime` must be called after `gtk::main()` now.
- `woab::close_actix_runtime` return an error instead of panicing if the
  runtime is closed or in use. This is breaking because now it is returnes two
  nested `Result`s, but it's a minor function that's not used all over the
  place, so it's not a big break.

### Added
- `woab::is_runtime_running`.

## 0.6.0 - 2021-07-10
### Changed
- [**BREAKING**] Upgraded gtk-rs to version 0.14. This is a breaking change
  because gtk-rs' API was changed in this release.
- Updated Actix version to 0.12.
- Code that triggers actor-handled signals is no longer required to trigger it
  from outside the Actix runtime, as long as the signal does not expect an
  inhibit decision and as long as it does not accept a context parameter.
- [**BREAKING**] `woab::run_actix_inside_gtk_event_loop` returns `()` instead
  of a `Result`.

### Added
- `event_param` method for `woab::Signal`, to easily get the concrete GDK event
  type from an event signal.
- `woab::close_actix_runtime` to shut down the Actix runtime.

### Fixed
- `woab::run_actix_inside_gtk_event_loop` was making `gtk::idle_add` busy-wait.
  Wait 10ms inside each such idle invocation to prevent that.

## 0.5.0 - 2021-05-19
### Added
- `#[derive(woab::PropSync)]` for generating getter and setter for relevant
  widgets' properties.

## 0.4.0 - 2021-02-15
### Added
- `BuilderConnectorWidgetsOnly` - a degraded version of `BuilderConnector` for
  getting widgets after the signals were connected.
- Facilities for working with GTK inside futures that run on the Actix runtime:
  - `woab::wake_from` for `await`ing to some signal somewhere.
    - `woab::wake_from_signal` variant that also disconnects the signal handler
      afterwards.
  - `woab::outside` for `await`ing on a future that runs outside the Actix runtime.
  - `woab::run_dialog` for as an `async` replacement for `gtk::DialogExt.run`
    that plays nice with WoAB.

### Changed
- [**BREAKING**] Changed `woab::schedule_outside` to `woab::spawn_outside`. The
  new function accepts a future, not a closure, and that future will run on GTK
  loop outside Actix.

## 0.3.0 - 2021-02-09
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
- `#[widget(nested)]` for nesting widget structs.
- `woab::schedule_outside` for running things that would fire signals outside
  the Actix runtime.
- `woab::params!` macro for extracting params from signals.

### Changed
- [**BREAKING**] Updated Actix to 0.11 and Tokio to 1.14. Consequences:
  - Actors can no longer just be started from outide Tokio/Actix. Instead, they
    must be started in a future (`async` block) passed the new `woab::block_on`
    function.
  - `woab::run_actix_inside_gtk_event_loop` no longer accepts a name.
- `BuilderConnector` is now consumes when the signals are routed.

### Removed
- [**BREAKING**] Removed everything related to `BuilderSignal` - the derive
  macro, the trait, and all the builder connector methods and helper structs.
  Use `woab::Signal` instead.
- [**REGRESSION**] Removed conversion of GDK events from `gdk::Event` to the
  concrete event type. Ability will be added again when possible - see #22.

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
