use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::task;
use tokio::time::{Duration, Instant};

#[derive(Copy, Clone)]
pub enum TimerType {
    Work,
    Break,
}

impl fmt::Display for TimerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match self {
            TimerType::Work => "work",
            TimerType::Break => "break",
        };

        write!(f, "{}", string)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum TimerEvent {
    Finish,
    Start,
    Pause,
    Stop,
}

#[derive(Copy, Clone)]
pub enum TimerState {
    Running,
    Paused,
    Stopped,
}

pub struct Timer {
    remaining: Duration,
    handle: Option<tokio::task::JoinHandle<()>>,
    state: TimerState,
    last_started_at: Option<Instant>,
    timer_type: TimerType,
    event_handlers: HashMap<TimerEvent, Vec<Arc<dyn Fn(&Timer) + Send + Sync>>>,
}

impl Timer {
    pub fn new(timer_type: TimerType, duration: &Duration) -> Arc<Mutex<Timer>> {
        Arc::new(Mutex::new(Timer {
            event_handlers: HashMap::new(),
            handle: None,
            last_started_at: None,
            remaining: duration.clone(),
            state: TimerState::Stopped,
            timer_type,
        }))
    }

    pub fn start(timer: &Arc<Mutex<Timer>>) {
        let timer = Arc::clone(timer);

        let mut timer_guard = timer.lock().expect("Failed to lock timer");

        if let TimerState::Running = timer_guard.state {
            return;
        }

        let duration = timer_guard.time_left();

        let timer2 = Arc::clone(&timer);

        let handle = task::spawn(async move {
            tokio::time::sleep(duration).await;

            let mut timer_guard = timer2.lock().expect("Failed to lock timer");
            timer_guard.finished();
        });

        timer_guard.handle = Some(handle);
        timer_guard.last_started_at = Some(Instant::now());
        timer_guard.state = TimerState::Running;

        timer_guard.event(TimerEvent::Start);
    }

    pub fn stop(&mut self) {
        self.state = TimerState::Stopped;

        // abort current sleep task
        self.abort_current_task();

        self.event(TimerEvent::Stop);
    }

    pub fn pause(&mut self) {
        println!("Timer.pause()");
        // it's only possible to pause a timer that's running
        if let TimerState::Running = self.state {
            self.remaining = self.time_left();

            // abort current sleep task
            self.abort_current_task();

            self.state = TimerState::Paused;

            self.event(TimerEvent::Pause);
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

        if elapsed <= self.remaining {
            self.remaining - elapsed
        } else {
            Duration::ZERO
        }
    }

    pub fn on(&mut self, event: TimerEvent, callback: Arc<dyn Fn(&Timer) + Send + Sync>) {
        self.event_handlers
            .entry(event)
            .or_default()
            .push(callback.clone());
    }

    pub fn timer_type(&self) -> TimerType {
        self.timer_type.clone()
    }

    pub fn clone(&self) -> Timer {
        Timer {
            handle: None,
            last_started_at: self.last_started_at.clone(),
            remaining: self.remaining.clone(),
            state: self.state.clone(),
            timer_type: self.timer_type.clone(),
            event_handlers: HashMap::new(),
        }
    }

    fn abort_current_task(&self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }

    fn finished(&mut self) {
        self.remaining = Duration::ZERO;
        self.state = TimerState::Stopped;

        self.event(TimerEvent::Finish);
    }

    fn event(&self, event: TimerEvent) {
        if let Some(handlers) = self.event_handlers.get(&event) {
            for callback in handlers {
                callback(&*self);
            }
        }
    }
}

// impl Drop for Timer {
//     fn drop(&mut self) {
//         println!("Timer dropped");
//     }
// }
