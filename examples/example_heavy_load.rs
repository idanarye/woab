use std::time::Instant;

use actix::prelude::*;
use gtk::prelude::*;

#[derive(woab::Factories)]
struct Factories {
    #[factory(extra(adj_num_rows))]
    win_app: woab::BuilderFactory,
    row: woab::BuilderFactory,
}

struct WindowActor {
    #[allow(dead_code)]
    factories: Factories,
    widgets: WindowWidgets,
    rows: Vec<actix::Addr<RowActor>>,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    win_app: gtk::ApplicationWindow,
    scl_num_rows: gtk::Scale,
    lst_rows: gtk::ListBox,
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "num_rows_slider_changed" => {
                let value = self.widgets.scl_num_rows.value() as usize;
                match value.cmp(&self.rows.len()) {
                    std::cmp::Ordering::Equal => (),
                    std::cmp::Ordering::Less => self.reduce_rows(value),
                    std::cmp::Ordering::Greater => self.increase_rows(value),
                }
                None
            }
            "close" => {
                gtk::main_quit();
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

impl WindowActor {
    fn reduce_rows(&mut self, num_rows: usize) {
        for row in self.rows.drain(num_rows..) {
            row.do_send(woab::Remove);
        }
    }

    fn increase_rows(&mut self, num_rows: usize) {
        self.rows.reserve(num_rows);
        for i in self.rows.len()..num_rows {
            self.factories.row.instantiate().connect_with(|bld| {
                let widgets: RowWidgets = bld.widgets().unwrap();
                self.widgets.lst_rows.add(&widgets.row);
                let actor = RowActor {
                    widgets,
                    position: 0.0,
                    velocity: 0.1 + (i as f64 * 0.001).sqrt(),
                    prev_update: Instant::now(),
                }
                .start();
                self.rows.push(actor.clone());
                actor.do_send(Step);
                actor
            });
        }
    }
}

#[derive(woab::Removable)]
#[removable(self.widgets.row)]
struct RowActor {
    widgets: RowWidgets,
    position: f64,
    velocity: f64,
    prev_update: Instant,
}

impl Actor for RowActor {
    type Context = actix::Context<Self>;
}

#[derive(woab::WidgetsFromBuilder)]
struct RowWidgets {
    row: gtk::ListBoxRow,
    draw_area: gtk::DrawingArea,
}

impl actix::Handler<woab::Signal> for RowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "draw" => {
                let woab::params! {
                    _,
                    draw_ctx: cairo::Context,
                } = msg.params()?;
                let area_size = self.widgets.draw_area.allocation();
                draw_ctx.arc(
                    self.position * area_size.width() as f64,
                    0.5 * area_size.height() as f64,
                    10.0,
                    0.0,
                    2.0 * std::f64::consts::PI,
                );
                draw_ctx.set_source_rgb(0.5, 0.5, 0.5);
                draw_ctx.fill().unwrap();
                Some(glib::Propagation::Stop)
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct Step;

impl actix::Message for Step {
    type Result = ();
}

impl actix::Handler<Step> for RowActor {
    type Result = ();

    fn handle(&mut self, _msg: Step, ctx: &mut Self::Context) -> Self::Result {
        let update_time = Instant::now();
        let frame_length = update_time - self.prev_update;
        self.prev_update = update_time;
        let new_position = self.position + self.velocity * frame_length.as_secs_f64();
        self.position = new_position % 1.0;
        self.widgets.draw_area.queue_draw();
        let addr = ctx.address();
        ctx.spawn(
            async move {
                let _ = addr.try_send(Step);
            }
            .into_actor(self),
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = Factories::read(std::io::BufReader::new(std::fs::File::open(
        "examples/example_heavy_load.glade",
    )?))?;

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop();

    woab::block_on(async {
        factories.win_app.instantiate().connect_with(|bld| {
            let widgets: WindowWidgets = bld.widgets().unwrap();
            widgets.win_app.show();
            WindowActor {
                factories,
                widgets,
                rows: Default::default(),
            }
            .start()
        });
    });
    gtk::main();
    woab::close_actix_runtime()??;
    Ok(())
}
