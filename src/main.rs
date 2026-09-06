mod layout;

use chrono::{Local, Timelike};
use clap::Parser;
use iced::keyboard::key;
use iced::widget::text::Wrapping;
use iced::widget::{Image, Space, Stack, column, container, image, responsive, text, text_input};
use iced::{
    Alignment, Background, Color, Element, Event, Length, Subscription, Task as Command, Theme,
    keyboard,
};
use layout::{LayoutMetrics, ScreenText, UserSizes};
use pam_unix::Client;
use std::{cell::RefCell, sync::LazyLock};
use uzers::{get_current_uid, get_user_by_uid};

use iced_exwlshell::sessionlock::application;
use iced_exwlshell::to_sessionlock_message;

const IMAGE_A: &[u8] = include_bytes!("../assets/app/wallpaper2.jpeg");
const IMAGE_B: &[u8] = include_bytes!("../assets/app/wallpaper1.jpeg");
const ACCOUNT: &[u8] = include_bytes!("../assets/app/account.png");

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

fn main() -> Result<(), iced_exwlshell::Error> {
    let args = Args::parse();
    let service = args.pam;

    if !std::path::Path::new(&format!("/etc/pam.d/{service}")).exists() {
        eprintln!("coldlock: PAM service '{service}' has no config in /etc/pam.d.");
        eprintln!("Refusing to lock.");
        std::process::exit(1);
    }

    application(move || Lock::new(service.clone()), Lock::update, Lock::view)
        .theme(Lock::theme)
        .subscription(Lock::subscription)
        .run()
}

struct Lock {
    steps: AuthSteps,
    aligned: bool,
}

#[to_sessionlock_message]
#[derive(Debug, Clone)]
enum Message {
    Step(StepMessage),
    EnterEvent(Event),
    Tick,
    UnLock,
}

impl Lock {
    fn new(service: String) -> (Self, Command<Message>) {
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
            Message::EnterEvent(event) => match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key::Named::Enter),
                    ..
                }) => {
                    self.steps.advance();
                    iced::widget::operation::focus(INPUT_ID.clone())
                }
                _ => Command::none(),
            },

            Message::UnLock => iced::exit(),

            Message::Tick => {
                self.aligned = Local::now().second() == 0;
                Command::none()
            }

            Message::Step(step_msg) => self.steps.update(step_msg),
        }
    }

    fn view(&'_ self) -> Element<'_, Message> {
        let Lock { steps, .. } = self;

        column![steps.view().map(Message::Step)].into()
    }
}

struct AuthSteps {
    steps: Vec<AuthStep>,

    current: usize,
}

impl AuthSteps {
    fn new(service: String) -> AuthSteps {
        let user = get_user_by_uid(get_current_uid()).unwrap();
        let user_name = user.name().to_string_lossy().into_owned();
        let icon_path = format!("/var/lib/AccountsService/icons/{user_name}");
        let icon_handle = match std::fs::read(icon_path) {
            Ok(data) => image::Handle::from_bytes(data),
            Err(_) => ACCOUNT_DEFAULT_HANDLE.clone(),
        };
        Self {
            steps: vec![
                AuthStep::Welcome {
                    icon_handle: icon_handle.clone(),
                    user_name: user_name.clone(),
                    layouts: RefCell::default(),
                },
                AuthStep::Auth {
                    icon_handle,
                    name: user_name,
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
        layouts: RefCell<Vec<(iced::Size, LayoutMetrics)>>,
    },
    Auth {
        icon_handle: image::Handle,
        name: String,
        password: String,
        auth_error: String,
        service: String,
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
                    let service = service.clone();
                    return Command::perform(
                        async move {
                            let mut client = Client::with_password(&service)
                                .expect("Failed to init PAM client.");
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
                layouts,
            } => Self::welcome(user_name, icon_handle.clone(), layouts),
            AuthStep::Auth {
                name,
                password,
                auth_error,
                icon_handle,
                service: _,
            } => Self::auth(name, password, auth_error, icon_handle.clone()),
        }
    }

    fn welcome<'b>(
        user_name: &'b str,
        user_icon: image::Handle,
        layouts: &'b RefCell<Vec<(iced::Size, LayoutMetrics)>>,
    ) -> Element<'b, StepMessage> {
        let now = Local::now();
        let day = now.format("%A, %B %e").to_string();
        let time = now.format("%H:%M").to_string();
        let name = format!("Welcome {user_name}");
        let foreground = responsive(move |size| {
            let metrics = {
                let mut layouts = layouts.borrow_mut();
                if let Some((_, metrics)) = layouts.iter().find(|(surface, _)| *surface == size) {
                    *metrics
                } else {
                    let metrics = layout::fit_layout(
                        size,
                        ScreenText::Welcome {
                            name: &name,
                            time: &time,
                            date: &day,
                        },
                    );
                    layouts.push((size, metrics));
                    metrics
                }
            };
            let sizes = UserSizes::new(metrics.scale);
            let clock_block = column![
                text(time.clone())
                    .font(layout::BOLD)
                    .size(75.0 * metrics.scale)
                    .line_height(layout::LINE_HEIGHT)
                    .wrapping(Wrapping::None),
                text(day.clone())
                    .font(layout::BOLD)
                    .size(35.0 * metrics.scale)
                    .line_height(layout::LINE_HEIGHT)
                    .wrapping(Wrapping::None),
            ]
            .spacing(5.0 * metrics.scale)
            .align_x(Alignment::Center);
            let details = column![
                Space::new().height(sizes.welcome_gap),
                text(name.clone())
                    .size(sizes.welcome_name_size)
                    .line_height(layout::LINE_HEIGHT)
                    .wrapping(Wrapping::WordOrGlyph)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
                Space::new().height(sizes.hint_gap),
                text(layout::WELCOME_HINT)
                    .size(sizes.hint_size)
                    .line_height(layout::LINE_HEIGHT)
                    .wrapping(Wrapping::WordOrGlyph)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            ]
            .width(Length::Fill);
            Stack::new()
                .push(group_at(metrics.clock_top, clock_block.into()))
                .push(user_block(user_icon.clone(), metrics, details.into()))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        });
        background(Image::new(IMAGE_B_HANDLE.clone()), foreground.into())
    }

    fn auth(
        name: &'a str,
        password: &'a str,
        auth_error: &'a str,
        user_icon: image::Handle,
    ) -> Element<'a, StepMessage> {
        let foreground = responsive(move |size| {
            let metrics = layout::fit_layout(
                size,
                ScreenText::Auth {
                    name,
                    error: auth_error,
                },
            );
            let sizes = UserSizes::new(metrics.scale);
            let scale = metrics.scale;
            let mut details = column![
                Space::new().height(sizes.auth_gap),
                text(name)
                    .size(sizes.auth_name_size)
                    .font(layout::BOLD)
                    .line_height(layout::LINE_HEIGHT)
                    .wrapping(Wrapping::WordOrGlyph)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
                Space::new().height(sizes.auth_gap),
                text_input("Enter password", password)
                    .padding(sizes.input_padding)
                    .line_height(layout::LINE_HEIGHT)
                    .style(move |_theme, _status| text_input::Style {
                        background: Background::Color(Color::from_rgb8(60, 60, 60)),
                        border: iced::Border {
                            color: Color::TRANSPARENT,
                            width: 2.0 * scale,
                            radius: (10.0 * scale).into(),
                        },
                        icon: Color::TRANSPARENT,
                        placeholder: Color::WHITE,
                        value: Color::WHITE,
                        selection: Color::from_rgb8(0, 150, 255),
                    })
                    .on_input(StepMessage::PasswordEntered)
                    .secure(true)
                    .id(INPUT_ID.clone())
                    .on_submit(StepMessage::Submit)
                    .width(sizes.input_width.min(metrics.details_width()))
                    .size(sizes.input_size),
            ]
            .width(Length::Fill)
            .align_x(Alignment::Center);
            if !auth_error.is_empty() {
                details = details.push(Space::new().height(sizes.auth_gap)).push(
                    text(auth_error)
                        .size(sizes.error_size)
                        .line_height(layout::LINE_HEIGHT)
                        .wrapping(Wrapping::WordOrGlyph)
                        .align_x(Alignment::Center)
                        .width(Length::Fill),
                );
            }
            user_block(user_icon.clone(), metrics, details.into())
        });
        background(Image::new(IMAGE_A_HANDLE.clone()), foreground.into())
    }
}

fn group_at(top: f32, content: Element<'_, StepMessage>) -> Element<'_, StepMessage> {
    column![Space::new().height(top), content]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

fn user_block(
    icon: image::Handle,
    metrics: LayoutMetrics,
    details: Element<'_, StepMessage>,
) -> Element<'_, StepMessage> {
    let group = column![
        Image::new(icon)
            .width(metrics.avatar_size)
            .height(metrics.avatar_size),
        container(details)
            .padding([0.0, layout::DETAILS_PADDING])
            .width(metrics.foreground_width)
            .height(metrics.details_height),
    ]
    .align_x(Alignment::Center);
    group_at(metrics.avatar_top, group.into())
}

fn background(image: Image, foreground: Element<'_, StepMessage>) -> Element<'_, StepMessage> {
    Stack::new()
        .push(
            image
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Cover),
        )
        .push(foreground)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
