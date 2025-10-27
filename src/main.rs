use coldlock::Lock;
use iced_sessionlock::build_pattern::application;

fn main() -> Result<(), iced_sessionlock::Error> {
    application(Lock::update, Lock::view)
        .theme(Lock::theme)
        .subscription(Lock::subscription)
        .run_with(Lock::new)
}
