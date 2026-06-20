use chrono::{Local, Timelike};
use clap::Parser;
use iced::keyboard::key;
use iced::widget::{Image, Stack, column, container, image, text, text_input};
use iced::window::Id;
use iced::{
    Alignment, Background, Color, Element, Event, Length, Subscription, Task as Command, Theme,
    keyboard,
};
use pam::Client;
use std::sync::LazyLock;
use uzers::{get_current_uid, get_user_by_uid};

use iced_sessionlock::to_session_message;

use iced_sessionlock::build_pattern::application;

const IMAGE_A: &[u8] = include_bytes!("../assets/wallpaper2.jpeg");
const IMAGE_B: &[u8] = include_bytes!("../assets/wallpaper1.jpeg");
const ACCOUNT: &[u8] = include_bytes!("../assets/account.png");

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

static IMAGE_A_HANDLE: LazyLock<image::Handle> =
    LazyLock::new(|| image::Handle::from_bytes(IMAGE_A));
static IMAGE_B_HANDLE: LazyLock<image::Handle> =
    LazyLock::new(|| image::Handle::from_bytes(IMAGE_B));
static ACCOUNT_DEFAULT_HANDLE: LazyLock<image::Handle> =
    LazyLock::new(|| image::Handle::from_bytes(ACCOUNT));

/// ColdLock — a Wayland session locker.
#[derive(clap::Parser)]
#[command(name = "coldlock", version, about)]
struct Args {
    /// PAM service to authenticate against (must have a config in /etc/pam.d).
    #[arg(long, default_value = "coldlock")]
    pam: String,
}

fn main() -> Result<(), iced_sessionlock::Error> {
    let args = Args::parse();
    let service: &'static str = Box::leak(args.pam.into_boxed_str());

    if !std::path::Path::new(&format!("/etc/pam.d/{service}")).exists() {
        eprintln!("coldlock: PAM service '{service}' has no config in /etc/pam.d.");
        eprintln!("Refusing to lock.");
        std::process::exit(1);
    }

    application(move || Lock::new(service), Lock::update, Lock::view)
        .theme(Lock::theme)
        .subscription(Lock::subscription)
        .run()
}

struct Lock {
    steps: AuthSteps,
    aligned: bool,
}

#[to_session_message]
#[derive(Debug, Clone)]
enum Message {
    NextPressed,
    Step(StepMessage),
    EnterEvent(Event),
    Tick,
}

impl Lock {
    fn new(service: &'static str) -> (Self, Command<Message>) {
        (
            Self {
                steps: AuthSteps::new(service),
                aligned: false,
            },
            Command::none(),
        )
    }

    fn theme(&self) -> iced::Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Message> {
        let period = if self.aligned { 60 } else { 1 };
        Subscription::batch(vec![
            iced::event::listen().map(Message::EnterEvent),
            iced::time::every(std::time::Duration::from_secs(period)).map(|_| Message::Tick),
        ])
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::NextPressed => {
                self.steps.advance();
                iced::widget::operation::focus(INPUT_ID.clone())
            }

            Message::EnterEvent(event) => match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key::Named::Enter),
                    ..
                }) => {
                    let message = Message::NextPressed;
                    Command::perform(async { message }, |msg| msg)
                }
                _ => Command::none(),
            },

            Message::UnLock => Command::done(message),

            Message::Tick => {
                self.aligned = Local::now().second() == 0;
                Command::none()
            }

            Message::Step(step_msg) => self.steps.update(step_msg),
        }
    }

    fn view(&'_ self, _window: Id) -> Element<'_, Message> {
        let Lock { steps, .. } = self;

        column![steps.view().map(Message::Step)].into()
    }
}

struct AuthSteps {
    steps: Vec<AuthStep>,

    current: usize,
}

impl AuthSteps {
    fn new(service: &'static str) -> AuthSteps {
        let user = get_user_by_uid(get_current_uid()).unwrap();
        let user_name = user.name().to_string_lossy().to_string().clone();
        let icon_path = format!("/var/lib/AccountsService/icons/{user_name}");
        let icon_path = std::path::Path::new(&icon_path);
        let icon_handle = if icon_path.exists() {
            if let Ok(data) = std::fs::read(icon_path) {
                image::Handle::from_bytes(data)
            } else {
                ACCOUNT_DEFAULT_HANDLE.clone()
            }
        } else {
            ACCOUNT_DEFAULT_HANDLE.clone()
        };
        Self {
            steps: vec![
                AuthStep::Welcome {
                    icon_handle: icon_handle.clone(),
                    user_name: user_name.clone(),
                },
                AuthStep::Auth {
                    icon_handle,
                    name: user_name.clone(),
                    password: String::new(),
                    auth_error: String::new(),
                    service,
                },
            ],
            current: 0,
        }
    }

    fn update(&mut self, msg: StepMessage) -> Command<Message> {
        self.steps[self.current].update(msg)
    }

    fn view(&'_ self) -> Element<'_, StepMessage> {
        self.steps[self.current].view()
    }

    fn advance(&mut self) {
        if self.can_continue() {
            self.current += 1;
        }
    }

    fn can_continue(&self) -> bool {
        self.current + 1 < self.steps.len() && self.steps[self.current].can_continue()
    }
}

enum AuthStep {
    Welcome {
        icon_handle: image::Handle,
        user_name: String,
    },
    Auth {
        icon_handle: image::Handle,
        name: String,
        password: String,
        auth_error: String,
        service: &'static str,
    },
}

#[derive(Clone, Debug)]
enum StepMessage {
    PasswordEntered(String),
    Submit,
    AuthError(String),
}

impl<'a> AuthStep {
    fn update(&mut self, msg: StepMessage) -> Command<Message> {
        match msg {
            StepMessage::AuthError(auth_error) => {
                if let AuthStep::Auth {
                    auth_error: error, ..
                } = self
                {
                    *error = auth_error;
                }
                Command::none()
            }

            StepMessage::PasswordEntered(password) => {
                if let AuthStep::Auth {
                    password: current_password,
                    ..
                } = self
                {
                    *current_password = password;
                }
                Command::none()
            }

            StepMessage::Submit => {
                if let AuthStep::Auth {
                    name,
                    password,
                    service,
                    ..
                } = self
                {
                    let name = name.clone();
                    let password = password.clone();
                    let service = *service;
                    return Command::perform(
                        async move {
                            let mut client =
                                Client::with_password(service).expect("Failed to init PAM client.");
                            client.conversation_mut().set_credentials(&name, &password);
                            client.authenticate()
                        },
                        |result| match result {
                            Ok(_) => Message::UnLock,
                            Err(e) => Message::Step(StepMessage::AuthError(format!("{}", e))),
                        },
                    );
                }
                Command::none()
            }
        }
    }

    fn can_continue(&self) -> bool {
        match self {
            AuthStep::Welcome { .. } => true,
            AuthStep::Auth { .. } => true,
        }
    }

    fn view(&'_ self) -> Element<'_, StepMessage> {
        match self {
            AuthStep::Welcome {
                user_name,
                icon_handle,
            } => Self::welcome(user_name, icon_handle.clone()),
            AuthStep::Auth {
                name,
                password,
                auth_error,
                icon_handle,
                service: _,
            } => Self::auth(name, password, auth_error, icon_handle.clone()),
        }
    }

    fn welcome(user_name: &'_ str, user_icon: image::Handle) -> Element<'_, StepMessage> {
        let image = Image::new(IMAGE_B_HANDLE.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Cover)
            .opacity(10.0);

        let now = Local::now();
        let day = now.format("%A, %B %e").to_string();
        let time = now.format("%H:%M").to_string();
        let col = column![
            text(time)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .size(75),
            text(day)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .size(35),
            iced::widget::Space::new().height(70),
            Image::new(user_icon)
                .width(Length::Fixed(120.))
                .height(Length::Fixed(120.)),
            text(format!("Welcome {}", user_name)).size(35),
            iced::widget::Space::new().height(30),
            text("Press Enter to unlock").size(22)
        ]
        .padding(220)
        .spacing(5)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);

        let st = Stack::new()
            .push(image)
            .push(col)
            .width(Length::Fill)
            .height(Length::Fill);
        container(st)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn auth(
        name: &'a str,
        password: &'a str,
        auth_error: &'a str,
        user_icon: image::Handle,
    ) -> Element<'a, StepMessage> {
        let col = column![
            Image::new(user_icon)
                .width(Length::Fixed(120.))
                .height(Length::Fixed(120.)),
            text(name).size(45).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            }),
            text_input("Enter password", password)
                .padding(10)
                .style(move |_theme, _status| text_input::Style {
                    background: Background::Color(Color::from_rgb8(60, 60, 60)),
                    border: iced::Border {
                        color: Color::TRANSPARENT,
                        width: 2.0,
                        radius: 10.0.into()
                    },
                    icon: Color::TRANSPARENT,
                    placeholder: Color::WHITE,
                    value: Color::WHITE,
                    selection: Color::from_rgb8(0, 150, 255)
                })
                .on_input(StepMessage::PasswordEntered)
                .secure(true)
                .id(INPUT_ID.clone())
                .on_submit(StepMessage::Submit)
                .width(Length::Fixed(320f32))
                .size(30),
            text(auth_error),
        ]
        .padding(350)
        .spacing(40)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);

        let image = Image::new(IMAGE_A_HANDLE.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Cover);

        let st = Stack::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .push(image)
            .push(col);

        container(st)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
