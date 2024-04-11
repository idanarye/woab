use core::time::Duration;

use actix::prelude::*;
use gtk4::prelude::*;
use send_wrapper::SendWrapper;

const BALL_RADIUS: f64 = 20.0;

struct WindowActor {
    area_size: [f64; 2],
    draw_area: gtk4::DrawingArea,
    ball: Ball,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "resize" => {
                let woab::params! {
                    _,
                    width: i32,
                    height: i32,
                } = msg.params()?;
                self.area_size = [width as f64, height as f64];
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct Draw(SendWrapper<cairo::Context>);

impl actix::Message for Draw {
    type Result = ();
}

impl actix::Handler<Draw> for WindowActor {
    type Result = ();

    fn handle(&mut self, msg: Draw, _ctx: &mut Self::Context) -> Self::Result {
        let draw_ctx = msg.0.take();
        draw_ctx.arc(
            self.ball.position[0],
            self.ball.position[1],
            BALL_RADIUS,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        draw_ctx.set_source_rgb(0.5, 0.5, 0.5);
        draw_ctx.fill().unwrap();
    }
}

struct Step(Duration);

impl actix::Message for Step {
    type Result = ();
}

impl actix::Handler<Step> for WindowActor {
    type Result = ();

    fn handle(&mut self, msg: Step, _ctx: &mut Self::Context) -> Self::Result {
        if self.area_size[0] <= 0.0 || self.area_size[1] <= 0.0 {
            return;
        }
        let Step(step_length) = msg;
        let step_length = step_length.as_secs_f64();

        let min_pos = [BALL_RADIUS, BALL_RADIUS];
        let max_pos = [self.area_size[0] - BALL_RADIUS, self.area_size[1] - BALL_RADIUS];
        self.ball.run_step(step_length, min_pos, max_pos);

        self.draw_area.queue_draw();
    }
}

struct Ball {
    position: [f64; 2],
    velocity: [f64; 2],
}

impl Ball {
    fn run_step(&mut self, step_length: f64, min_pos: [f64; 2], max_pos: [f64; 2]) {
        for coord in 0..2 {
            let mut position = self.position[coord];
            let velocity = self.velocity[coord];
            position += velocity * step_length;
            if (0.0 < velocity && max_pos[coord] < position) || (velocity < 0.0 && position < min_pos[coord]) {
                self.velocity[coord] = -velocity;
            } else {
                self.position[coord] = position;
            }
        }
    }
}

fn main() -> woab::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_canvas.ui")?);

    woab::main(Default::default(), move |app| {
        woab::shutdown_when_last_window_is_closed(app);
        WindowActor::create(|ctx| {
            let bld = factory.instantiate_route_to(ctx.address());
            bld.set_application(app);
            bld.get_object::<gtk4::ApplicationWindow>("win_app").unwrap().show();

            actix::spawn({
                let addr = ctx.address();
                async move {
                    use actix::clock::Instant;
                    let mut last_step_time = Instant::now();
                    loop {
                        let step_time = Instant::now();
                        addr.send(Step(step_time - last_step_time)).await.unwrap();
                        last_step_time = step_time;
                    }
                }
            });

            let draw_area: gtk4::DrawingArea = bld.get_object("draw_area").unwrap();

            let addr = ctx.address();
            draw_area.set_draw_func(move |_, draw_ctx, _, _| {
                woab::block_on(addr.send(Draw(SendWrapper::new(draw_ctx.clone())))).unwrap();
            });

            WindowActor {
                area_size: [0.0, 0.0],
                draw_area,
                ball: Ball {
                    position: [BALL_RADIUS * 2.0, BALL_RADIUS * 2.0],
                    velocity: [100.0, 100.0],
                },
            }
        });
    })
}
