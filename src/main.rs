//! keepy-uppy: keep a MacBook awake with the lid closed.

mod config;
mod icons;
mod platform;
mod policy;
mod tray;
mod update;
mod watchdog;

use std::time::{Duration, Instant};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::MenuEvent;

use config::Config;
use platform::{Controller, SleepController};
use policy::{Decision, Inputs, Reason, Session};
use tray::{Action, Tray, View};
use update::UpdateState;
use watchdog::Watchdog;

/// powerd can clear our bit at any time (docs/mechanism.md), so re-arm this often.
const TICK: Duration = Duration::from_secs(1);

enum UserEvent {
    Menu(MenuEvent),
    Update(Result<Option<String>, String>),
}

struct App {
    controller: Controller,
    watchdog: Watchdog,
    session: Session,
    timer_hours: Option<u64>,
    last_reason: Reason,
    config: Config,
    chime_muted: bool,
    launch_at_login: bool,
    update: UpdateState,
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
            low_battery: self.config.low_battery(),
            thermal: platform::thermal(),
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
        self.watchdog.report(self.controller.is_armed());
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
            Action::LowBattery(limit) => {
                self.config.set_low_battery(limit);
                if let Err(e) = self.config.save() {
                    self.error = Some(e);
                }
            }
            Action::IconStyle(style) => {
                self.config.icon_style = style;
                if let Err(e) = self.config.save() {
                    self.error = Some(e);
                }
            }
            Action::ToggleChime => {
                if let Err(e) = platform::set_chime_muted(!self.chime_muted) {
                    self.error = Some(e);
                }
                self.chime_muted = platform::chime_muted();
            }
            Action::ToggleLaunchAtLogin => {
                if let Err(e) = platform::set_launch_at_login(!self.launch_at_login) {
                    self.error = Some(e);
                }
                self.launch_at_login = platform::launch_at_login();
            }
            Action::CheckForUpdates | Action::Quit => unreachable!("handled by the event loop"),
        }
    }

    fn quit(&mut self) {
        if self.controller.is_armed() {
            if let Err(e) = self.controller.disarm() {
                eprintln!("keepy-uppy: {e}");
            }
        }
        self.watchdog.report(self.controller.is_armed());
    }

    fn view(&self, now: Instant) -> View<'_> {
        View {
            session: self.session,
            timer_hours: self.timer_hours,
            armed: self.controller.is_armed(),
            now,
            last_reason: self.last_reason,
            low_battery: self.config.low_battery(),
            icon_style: self.config.icon_style,
            chime_muted: self.chime_muted,
            launch_at_login: self.launch_at_login,
            update: &self.update,
            error: self.error.as_deref(),
        }
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some(watchdog::ARG) {
        watchdog::run();
    }
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
        watchdog: Watchdog::start(),
        session: Session::Off,
        timer_hours: None,
        last_reason: Reason::Off,
        config: Config::load(),
        chime_muted: platform::chime_muted(),
        launch_at_login: platform::launch_at_login(),
        update: UpdateState::Idle,
        error: None,
    };

    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    let proxy = event_loop.create_proxy();
    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = menu_proxy.send_event(UserEvent::Menu(e));
    }));

    let mut tray: Option<Tray> = None;
    let mut next_tick = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        let now = Instant::now();
        match event {
            Event::NewEvents(StartCause::Init) => {
                // AppKit resets SIGTERM while launching, so install once running.
                platform::install_stop_signals();
                match Tray::new() {
                    Ok(t) => tray = Some(t),
                    Err(e) => {
                        eprintln!("keepy-uppy: tray: {e}");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                if platform::stop_requested() {
                    app.quit();
                    tray = None;
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
            Event::UserEvent(UserEvent::Menu(e)) => {
                match tray.as_ref().and_then(|t| t.action(&e.id)) {
                    Some(Action::Quit) => {
                        app.quit();
                        tray = None;
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    Some(Action::CheckForUpdates) => {
                        if let UpdateState::Available(tag) = &app.update {
                            if let Err(e) = platform::open_url(&update::release_page(tag)) {
                                app.error = Some(e);
                            }
                        } else {
                            app.update = UpdateState::Checking;
                            let proxy = proxy.clone();
                            std::thread::spawn(move || {
                                let _ = proxy.send_event(UserEvent::Update(update::check()));
                            });
                        }
                    }
                    Some(action) => app.handle(action, now),
                    None => return,
                }
            }
            Event::UserEvent(UserEvent::Update(result)) => {
                app.update = match result {
                    Ok(Some(tag)) => UpdateState::Available(tag),
                    Ok(None) => UpdateState::UpToDate,
                    Err(e) => {
                        eprintln!("keepy-uppy: {e}");
                        UpdateState::Failed
                    }
                };
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
