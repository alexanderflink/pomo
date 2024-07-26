use crate::timer::{Timer, TimerEvent, TimerType};
use flume;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::task;

#[derive(Clone)]
pub struct Config {
    pub work_duration: Duration,
    pub break_duration: Duration,
    pub long_break_duration: Duration,
    pub long_break_interval: u64,
    pub auto: bool,
}

pub struct Controller {
    timer: Arc<Mutex<Timer>>,
    tx: flume::Sender<String>,
    rx: flume::Receiver<String>,
    config: Config,
    event_handlers: HashMap<TimerEvent, Vec<Arc<dyn Fn(&Timer) + Send + Sync>>>,
    num_finished_timers: u64,
}

impl Controller {
    pub fn new(config: Config) -> Arc<Mutex<Controller>> {
        let (tx, rx) = flume::unbounded();

        let Config { work_duration, .. } = config;

        let timer = Controller::create_timer(tx.clone(), TimerType::Work, work_duration);

        Arc::new(Mutex::new(Controller {
            config,
            timer,
            tx,
            rx,
            event_handlers: HashMap::new(),
            num_finished_timers: 0,
        }))
    }

    fn create_timer(
        tx: flume::Sender<String>,
        timer_type: TimerType,
        duration: Duration,
    ) -> Arc<Mutex<Timer>> {
        // create a new timer
        let timer = Timer::new(timer_type, &duration.clone());

        let mut timer_guard = timer.lock().expect("Failed to lock timer");

        // add Finish event handler (only used for auto mode to start next timer)
        timer_guard.on(
            TimerEvent::Finish,
            Arc::new(move |_: &Timer| {
                tx.send("timer_finished".to_string())
                    .expect("Failed to send timer finished message");
            }),
        );

        drop(timer_guard);

        timer
    }

    fn attach_timer_handlers(&self) {
        let mut timer = self.timer.lock().expect("Failed to lock timer");

        // attach saved event handlers to timer
        for (event, handlers) in &self.event_handlers {
            for handler in handlers {
                timer.on(*event, handler.clone());
            }
        }
    }

    pub fn start(controller: &Arc<Mutex<Self>>) {
        let controller = Arc::clone(controller);

        let mut controller_guard_1 = controller.lock().expect("Failed to lock controller");
        let rx = controller_guard_1.rx.clone();

        controller_guard_1.start_current_timer();
        drop(controller_guard_1);

        task::spawn(async move {
            loop {
                let msg = rx
                    .recv_async()
                    .await
                    .expect("Failed to listen to socket messages");

                let mut controller_guard_2 = controller.lock().expect("Failed to lock controller");

                match msg.as_str() {
                    "timer_finished" => {
                        controller_guard_2.on_timer_finished();
                    }
                    "skip" => {
                        controller_guard_2.start_next_timer();
                    }
                    _ => {}
                }

                drop(controller_guard_2);
            }
        });
    }

    // TODO: These methods look like they can be refactored into a single method
    pub fn next(controller: &Arc<Mutex<Self>>) {
        let mut controller = controller.lock().expect("Failed to lock controller");
        controller.start_next_timer();
    }

    pub fn stop(controller: &Arc<Mutex<Self>>) {
        let mut controller = controller.lock().expect("Failed to lock controller");
        controller.stop_current_timer();
    }

    pub fn pause(controller: &Arc<Mutex<Self>>) {
        let mut controller = controller.lock().expect("Failed to lock controller");
        controller.pause_current_timer();
    }

    fn start_current_timer(&mut self) {
        Timer::start(&self.timer);
    }

    fn stop_current_timer(&mut self) {
        let mut timer = self.timer.lock().expect("Failed to lock timer");
        timer.stop();
    }

    fn pause_current_timer(&mut self) {
        let mut timer = self.timer.lock().expect("Failed to lock timer");
        timer.pause();
    }

    fn start_next_timer(&mut self) {
        self.num_finished_timers += 1;

        self.stop_current_timer();

        let current_timer = self.timer.lock().expect("Failed to lock timer");

        let new_timer = match current_timer.timer_type() {
            TimerType::Work => {
                let mut duration = self.config.break_duration.clone();

                let num_finished_break_timers = self.num_finished_timers / 2;

                if num_finished_break_timers != 0
                    && num_finished_break_timers % (self.config.long_break_interval - 1) == 0
                {
                    duration = self.config.long_break_duration.clone();
                }

                Controller::create_timer(self.tx.clone(), TimerType::Break, duration)
            }
            TimerType::Break => Controller::create_timer(
                self.tx.clone(),
                TimerType::Work,
                self.config.work_duration.clone(),
            ),
        };

        drop(current_timer);

        self.timer = new_timer;

        self.attach_timer_handlers();

        self.start_current_timer();
    }

    fn on_timer_finished(&mut self) {
        // if Controller is in auto mode, start next timer
        if self.config.auto {
            self.start_next_timer();
            return;
        }
    }

    pub fn get_current_timer(controller: &Arc<Mutex<Self>>) -> Arc<Mutex<Timer>> {
        let controller = Arc::clone(controller);

        let controller = controller.lock().expect("Failed to lock controller");
        let timer = Arc::clone(&controller.timer);
        // drop(controller);
        timer
    }

    pub fn on(
        controller: &Arc<Mutex<Self>>,
        event: TimerEvent,
        callback: Arc<dyn Fn(&Timer) + Send + Sync>,
    ) {
        let mut controller = controller.lock().expect("Failed to lock controller");

        // save the handler for future timers
        controller
            .event_handlers
            .entry(event)
            .or_default()
            .push(callback.clone());

        // attach event listener to current timer
        let mut timer = controller.timer.lock().expect("Failed to lock timer");

        // attach the listener to the current timer
        timer.on(event, callback);
    }
}
