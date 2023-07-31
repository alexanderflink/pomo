use std::sync::Arc;

use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::task;

#[derive(Clone)]
pub enum TimerType {
    Work,
    Break,
}

#[derive(Clone)]
pub enum TimerState {
    Running,
    Paused,
    Stopped,
}

pub struct Timer {
    remaining: Duration,
    handle: Option<tokio::task::JoinHandle<()>>,
    on_finished_callback: Option<Arc<dyn Fn() + Send + Sync>>,
    state: TimerState,
    last_started_at: Option<Instant>,
    timer_type: TimerType,
}

impl Timer {
    pub fn new(timer_type: TimerType, duration: &Duration) -> Arc<Mutex<Timer>> {
        Arc::new(Mutex::new(Timer {
            remaining: duration.clone(),
            handle: None,
            on_finished_callback: None,
            last_started_at: None,
            state: TimerState::Stopped,
            timer_type,
        }))
    }

    pub fn start(timer: &Arc<Mutex<Timer>>) {
        println!("Timer.start()");
        let timer = Arc::clone(timer);

        let mut timer_guard = timer.lock().unwrap();

        let duration = timer_guard.time_left();

        println!("duration: {:?}", duration);

        let timer2 = Arc::clone(&timer);

        let handle = task::spawn(async move {
            tokio::time::sleep(duration).await;

            let mut timer_guard = timer2.lock().unwrap();
            timer_guard.finished();
        });

        timer_guard.handle = Some(handle);
        timer_guard.last_started_at = Some(Instant::now());
        timer_guard.state = TimerState::Running;
    }

    pub fn stop(&mut self) {
        println!("Timer.stop()");
        self.state = TimerState::Stopped;

        // abort current sleep task
        self.abort_current_task();
    }

    pub fn pause(&mut self) {
        println!("Timer.pause()");

        // it's only possible to pause a timer that's running
        if let TimerState::Running = self.state {
            self.remaining = self.time_left();

            // abort current sleep task
            self.abort_current_task();

            self.state = TimerState::Paused;
        }
    }

    pub fn time_left(&self) -> Duration {
        let elapsed = match &self.state {
            TimerState::Running => match self.last_started_at {
                Some(last_started_at) => last_started_at.elapsed(),
                None => Duration::ZERO,
            },
            _ => Duration::ZERO,
        };

        let remaining = self.remaining - elapsed;

        println!("Timer.time_left(): {:?}", remaining);

        remaining
    }

    pub fn on_finished(&mut self, callback: impl Fn() + 'static + Send + Sync) {
        self.on_finished_callback = Some(Arc::new(callback));
    }

    pub fn timer_type(&self) -> TimerType {
        self.timer_type.clone()
    }

    pub fn clone(&self) -> Timer {
        Timer {
            state: self.state.clone(),
            remaining: self.remaining.clone(),
            handle: None,
            on_finished_callback: None,
            last_started_at: self.last_started_at.clone(),
            timer_type: self.timer_type.clone(),
        }
    }

    fn abort_current_task(&self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }

    fn finished(&mut self) {
        println!("Timer.finished()");
        self.remaining = Duration::ZERO;
        self.state = TimerState::Stopped;

        if let Some(callback) = &self.on_finished_callback {
            callback();
        }
    }
}

// impl Drop for Timer {
//     fn drop(&mut self) {
//         println!("Timer dropped");
//     }
// }
