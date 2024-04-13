use actix::prelude::*;
use gtk4::prelude::*;

struct WindowActor {
    widgets: WindowWidgets,
}

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct WindowWidgets {
    #[prop_sync("value": f64, get, set)]
    adj_timer: gtk4::Adjustment,
    #[prop_sync(get, set)]
    txt_shortening: gtk4::Entry,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                // gtk4::main_quit();
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct Step;

impl actix::Message for Step {
    type Result = ();
}

impl actix::Handler<Step> for WindowActor {
    type Result = ();

    fn handle(&mut self, _msg: Step, _ctx: &mut Self::Context) -> Self::Result {
        let WindowWidgetsPropGetter {
            adj_timer,
            mut txt_shortening,
        } = self.widgets.get_props();
        if !txt_shortening.is_empty() {
            txt_shortening.remove(0);
        }
        self.widgets.set_props(&WindowWidgetsPropSetter {
            adj_timer: adj_timer + 1.0,
            txt_shortening: &txt_shortening,
        });
    }
}

fn main() -> woab::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_prop_sync.ui")?);

    woab::main(Default::default(), move |app| {
        woab::shutdown_when_last_window_is_closed(app);
        WindowActor::create(|ctx| {
            let bld = factory.instantiate_route_to(ctx.address());
            bld.set_application(app);
            bld.get_object::<gtk4::ApplicationWindow>("win_app").unwrap().show();

            let addr = ctx.address();
            let mut next_step = actix::clock::Instant::now();
            let step_duration = std::time::Duration::from_secs(1);
            actix::spawn(async move {
                loop {
                    next_step += step_duration;
                    actix::clock::sleep_until(next_step).await;
                    addr.send(Step).await.unwrap();
                }
            });
            WindowActor {
                widgets: bld.widgets().unwrap(),
            }
        });
    })
}
