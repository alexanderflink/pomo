use crate::timer::Timer;
use crate::timer::TimerType;
use flume;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::task;

pub struct Controller {
    timer: Arc<Mutex<Timer>>,
    tx: flume::Sender<String>,
    rx: flume::Receiver<String>,
    work_duration: Duration,
    break_duration: Duration,
    auto: bool,
}

impl Controller {
    pub fn new(
        work_duration: &Duration,
        break_duration: &Duration,
        auto: bool,
    ) -> Arc<Mutex<Controller>> {
        let (tx, rx) = flume::unbounded();

        let timer = Controller::create_timer(tx.clone(), TimerType::Work, work_duration);

        Arc::new(Mutex::new(Controller {
            auto,
            timer,
            tx,
            rx,
            work_duration: work_duration.clone(),
            break_duration: break_duration.clone(),
        }))
    }

    fn create_timer(
        tx: flume::Sender<String>,
        timer_type: TimerType,
        duration: &Duration,
    ) -> Arc<Mutex<Timer>> {
        // create a new timer
        let timer = Timer::new(timer_type, &duration.clone());

        let mut timer_guard = timer.lock().unwrap();

        timer_guard.on_finished(move || {
            tx.send("timer_finished".to_string()).unwrap();
        });

        drop(timer_guard);

        timer
    }

    pub fn start(controller: &Arc<Mutex<Self>>) {
        let controller = Arc::clone(controller);

        println!("Controller.start()");

        let mut controller_guard_1 = controller.lock().unwrap();
        let rx = controller_guard_1.rx.clone();

        controller_guard_1.start_current_timer();
        drop(controller_guard_1);

        task::spawn(async move {
            loop {
                let msg = rx.recv_async().await.unwrap();
                println!("1. Received message: {}", msg);

                let mut controller_guard_2 = controller.lock().unwrap();

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

    pub fn next(controller: &Arc<Mutex<Self>>) {
        let mut controller = controller.lock().unwrap();
        controller.start_next_timer();
    }

    pub fn stop(controller: &Arc<Mutex<Self>>) {
        let mut controller = controller.lock().unwrap();
        controller.stop_current_timer();
    }

    pub fn pause(controller: &Arc<Mutex<Self>>) {
        let mut controller = controller.lock().unwrap();
        controller.pause_current_timer();
    }

    fn start_current_timer(&mut self) {
        Timer::start(&self.timer);
    }

    fn stop_current_timer(&mut self) {
        let mut timer = self.timer.lock().unwrap();
        timer.stop();
    }

    fn pause_current_timer(&mut self) {
        let mut timer = self.timer.lock().unwrap();
        timer.pause();
    }

    fn start_next_timer(&mut self) {
        println!("Starting next timer!");
        self.stop_current_timer();

        let current_timer = self.timer.lock().unwrap();

        let new_timer = match current_timer.timer_type() {
            TimerType::Work => Controller::create_timer(
                self.tx.clone(),
                TimerType::Break,
                &self.break_duration.clone(),
            ),
            TimerType::Break => Controller::create_timer(
                self.tx.clone(),
                TimerType::Work,
                &self.work_duration.clone(),
            ),
        };

        drop(current_timer);

        self.timer = new_timer;

        self.start_current_timer();
    }

    fn on_timer_finished(&mut self) {
        println!("Timer finished!");

        if self.auto {
            self.start_next_timer();
            return;
        }

        // wait for user input or a skip message
        let tx = self.tx.clone();

        task::spawn(async move {
            println!("Press enter to start the next timer.");
            let _ = std::io::stdin().read_line(&mut String::new());
            println!("Yes!");
            tx.send("skip".to_string()).unwrap();
            println!("Sent skip message!");
        });
    }

    pub fn get_current_timer(controller: &Arc<Mutex<Self>>) -> Timer {
        let controller = Arc::clone(controller);

        let controller = controller.lock().unwrap();
        let timer = controller.timer.lock().unwrap().clone();
        drop(controller);
        timer
    }
}
