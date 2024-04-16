use std::time::Instant;

use actix::prelude::*;
use gtk4::prelude::*;
use send_wrapper::SendWrapper;

#[derive(woab::Factories)]
struct Factories {
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
    win_app: gtk4::ApplicationWindow,
    scl_num_rows: gtk4::Scale,
    #[allow(unused)]
    lst_rows: gtk4::ListBox,
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
            RowActor::create(|ctx| {
                let bld = self.factories.row.instantiate_without_routing_signals();
                let widgets: RowWidgets = bld.widgets().unwrap();
                let addr = ctx.address();
                widgets.draw_area.set_draw_func(move |_, draw_ctx, _, _| {
                    woab::block_on(addr.send(Draw(SendWrapper::new(draw_ctx.clone())))).unwrap();
                });
                self.widgets.lst_rows.append(&widgets.row);
                self.rows.push(ctx.address().clone());
                ctx.address().do_send(Step);
                RowActor {
                    widgets,
                    position: 0.0,
                    velocity: 0.1 + (i as f64 * 0.001).sqrt(),
                    prev_update: Instant::now(),
                }
            });
        }
    }
}

#[derive(woab::Removable)]
#[removable(self.widgets.row in gtk4::ListBox)]
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
    #[allow(unused)]
    row: gtk4::ListBoxRow,
    draw_area: gtk4::DrawingArea,
}

struct Draw(SendWrapper<cairo::Context>);

impl actix::Message for Draw {
    type Result = ();
}

impl actix::Handler<Draw> for RowActor {
    type Result = ();

    fn handle(&mut self, msg: Draw, _ctx: &mut Self::Context) -> Self::Result {
        let draw_ctx = msg.0.take();
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

fn main() -> woab::Result<()> {
    woab::main(Default::default(), move |app| {
        let factories = Factories::read(std::io::BufReader::new(
            std::fs::File::open("examples/example_heavy_load.ui").unwrap(),
        ))
        .unwrap();

        woab::shutdown_when_last_window_is_closed(app);
        WindowActor::create(|ctx| {
            let bld = factories.win_app.instantiate_route_to(ctx.address());
            bld.set_application(app);
            let widgets: WindowWidgets = bld.widgets().unwrap();
            widgets.win_app.show();
            WindowActor {
                factories,
                widgets,
                rows: Default::default(),
            }
        });
    })
}
