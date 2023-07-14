use async_recursion::async_recursion;
use core::time::Duration;
use tokio::task;
use tokio::task::JoinError;

enum TimerType {
    Work,
    Break,
}

struct Timer {
    timer_type: TimerType,
    duration: Duration,
}

impl Timer {
    fn new(timer_type: TimerType, duration: Duration) -> Timer {
        Timer {
            timer_type,
            duration,
        }
    }

    async fn start(&self) -> Result<(), JoinError> {
        // sleep in a tokio task for timer duration, then call self.on_finished
        let duration = self.duration.clone();
        println!("Starting timer for {:?}", duration);

        task::spawn(async move {
            tokio::time::sleep(duration).await;
            println!("Timer finished!");
        })
        .await
    }
}

struct TimersController {
    current_timer: Timer,
}

impl TimersController {
    fn new() -> Self {
        TimersController {
            current_timer: Timer::new(TimerType::Work, Duration::from_secs(5)),
        }
    }

    async fn start(&mut self) {
        self.current_timer.start().await;
        self.timer_finished().await;
    }

    #[async_recursion]
    async fn timer_finished(&mut self) {
        self.current_timer = Timer::new(TimerType::Break, Duration::from_secs(5));
        self.start().await;
    }

    fn get_current_timer(&self) -> &Timer {
        &self.current_timer
    }
}

#[tokio::main]
async fn main() {
    // create a controller which will manage timers
    let mut timers_controller = TimersController::new();

    task::spawn(async {
        // NEEDS IMPLEMENTATION

        // listen for incoming messages on a socket
        // if message equals "status", get current timer using timers_controller.get_current_timer
        // respond with time remaining
    });

    // start the current timer, this will await until all timers have been run
    timers_controller.start().await;
}
