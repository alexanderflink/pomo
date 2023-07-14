use core::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;

enum TimerType {
    Work,
    Break,
}

struct Timer {
    timer_type: TimerType,
    duration: Duration,
    // callback: Option<Box<dyn FnMut() + Sync + Send>>,
    callback: Option<
        Box<dyn FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send>,
    >,
}

impl Timer {
    fn new(timer_type: TimerType, duration: Duration) -> Timer {
        Timer {
            timer_type,
            duration,
            callback: None,
        }
    }

    fn start(timer: Arc<Mutex<Timer>>) {
        // sleep in a separate tokio task for timer duration, then call self.on_finished
        task::spawn(async move {
            let mut timer_guard = timer.lock().await;
            tokio::time::sleep(timer_guard.duration).await;
            timer_guard.callback.as_mut().unwrap()();
        });
    }

    fn on_finished(
        &mut self,
        callback: impl FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>
            + 'static
            + Send,
    ) {
        self.callback = Some(Box::new(callback));
    }
}

#[tokio::main]
async fn main() {
    let timer = Arc::new(Mutex::new(Timer::new(
        TimerType::Work,
        Duration::from_secs(60),
    )));

    Timer::start(Arc::clone(&timer));

    let timer = Arc::clone(&timer);
    let mut timer_guard = timer.lock().await;

    let timer = Arc::clone(&timer);
    timer_guard.on_finished(move || {
        let timer = Arc::clone(&timer);
        Box::pin(async move {
            let mut timer_guard = timer.lock().await;
            *timer_guard = Timer::new(TimerType::Break, Duration::from_secs(60));
            println!("Timer finished!");
        })
    });
}
