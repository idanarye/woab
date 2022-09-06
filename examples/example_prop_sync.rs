use actix::prelude::*;
use gtk::prelude::*;

struct WindowActor {
    widgets: WindowWidgets,
}

#[derive(woab::WidgetsFromBuilder, woab::PropSync)]
struct WindowWidgets {
    #[prop_sync("value": f64, get, set)]
    adj_timer: gtk::Adjustment,
    #[prop_sync(get, set)]
    txt_shortening: gtk::Entry,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                gtk::main_quit();
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_prop_sync.glade")?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop();

    woab::block_on(async {
        factory.instantiate().connect_with(|bld| {
            let win_app: gtk::ApplicationWindow = bld.get_object("win_app").unwrap();

            win_app.show();
            let addr = WindowActor {
                widgets: bld.widgets().unwrap(),
            }
            .start();

            actix::spawn({
                use actix::clock::Instant;
                let addr = addr.clone();
                let mut next_step = Instant::now();
                let step_duration = std::time::Duration::from_secs(1);
                async move {
                    loop {
                        next_step += step_duration;
                        actix::clock::sleep_until(next_step).await;
                        addr.send(Step).await.unwrap();
                    }
                }
            });

            addr
        });
    });

    gtk::main();
    woab::close_actix_runtime()??;
    Ok(())
}
