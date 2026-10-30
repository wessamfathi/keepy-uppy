//! keepy-uppy: keep a MacBook awake with the lid closed.

mod platform;
mod policy;
mod tray;

use std::time::{Duration, Instant};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::MenuEvent;

use platform::{Controller, SleepController};
use policy::{Decision, Inputs, Reason, Session, Thermal};
use tray::{Action, Tray, View};

/// powerd can clear our bit at any time (docs/mechanism.md), so re-arm this often.
const TICK: Duration = Duration::from_secs(1);
const DEFAULT_LOW_BATTERY: Option<u8> = Some(10);

enum UserEvent {
    Menu(MenuEvent),
}

struct App {
    controller: Controller,
    session: Session,
    timer_hours: Option<u64>,
    last_reason: Reason,
    low_battery: Option<u8>,
    chime_muted: bool,
    error: Option<String>,
}

impl App {
    /// Applies the policy. Arms on every call; disarms only on a transition so
    /// we never overwrite the bit while powerd is using it.
    fn step(&mut self, now: Instant) {
        let decision = policy::decide(&Inputs {
            session: self.session,
            now,
            power: platform::power(),
            low_battery: self.low_battery,
            thermal: Thermal::Nominal,
        });
        let result = match decision {
            Decision::Arm => self.controller.arm(),
            Decision::Disarm(reason) => {
                // A safety disarm ends the session; the user re-arms explicitly.
                // Later ticks then report `Off`, which must not hide the cause.
                if reason != Reason::Off {
                    self.last_reason = reason;
                }
                self.session = Session::Off;
                if self.controller.is_armed() {
                    self.controller.disarm()
                } else {
                    Ok(())
                }
            }
        };
        if let Err(e) = result {
            eprintln!("keepy-uppy: {e}");
            self.session = Session::Off;
            self.error = Some(e);
        }
    }

    fn handle(&mut self, action: Action, now: Instant) {
        self.error = None;
        match action {
            Action::Arm(hours) => {
                self.timer_hours = hours;
                self.session = match hours {
                    Some(h) => Session::Until(now + Duration::from_secs(h * 3600)),
                    None => Session::Indefinite,
                };
            }
            Action::Disarm => {
                self.session = Session::Off;
                self.last_reason = Reason::Off;
            }
            Action::LowBattery(limit) => self.low_battery = limit,
            Action::ToggleChime => {
                if let Err(e) = platform::set_chime_muted(!self.chime_muted) {
                    self.error = Some(e);
                }
                self.chime_muted = platform::chime_muted();
            }
            Action::Quit => unreachable!("handled by the event loop"),
        }
    }

    fn view(&self, now: Instant) -> View<'_> {
        View {
            session: self.session,
            timer_hours: self.timer_hours,
            armed: self.controller.is_armed(),
            now,
            last_reason: self.last_reason,
            low_battery: self.low_battery,
            chime_muted: self.chime_muted,
            error: self.error.as_deref(),
        }
    }
}

fn main() {
    let mut controller = match Controller::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("keepy-uppy: {e}");
            std::process::exit(1);
        }
    };
    // Always start disarmed: the kernel keeps the bit after a crash or kill -9.
    if let Err(e) = controller.disarm() {
        eprintln!("keepy-uppy: {e}");
    }
    let mut app = App {
        controller,
        session: Session::Off,
        timer_hours: None,
        last_reason: Reason::Off,
        low_battery: DEFAULT_LOW_BATTERY,
        chime_muted: platform::chime_muted(),
        error: None,
    };

    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(UserEvent::Menu(e));
    }));

    let mut tray: Option<Tray> = None;
    let mut next_tick = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        let now = Instant::now();
        match event {
            Event::NewEvents(StartCause::Init) => match Tray::new() {
                Ok(t) => tray = Some(t),
                Err(e) => {
                    eprintln!("keepy-uppy: tray: {e}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            },
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {}
            Event::UserEvent(UserEvent::Menu(e)) => {
                match tray.as_ref().and_then(|t| t.action(&e.id)) {
                    Some(Action::Quit) => {
                        if app.controller.is_armed() {
                            let _ = app.controller.disarm();
                        }
                        tray = None;
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    Some(action) => app.handle(action, now),
                    None => return,
                }
            }
            _ => {
                *control_flow = ControlFlow::WaitUntil(next_tick);
                return;
            }
        }
        app.step(now);
        if let Some(t) = tray.as_mut() {
            t.render(&app.view(now));
        }
        next_tick = now + TICK;
        *control_flow = ControlFlow::WaitUntil(next_tick);
    });
}
