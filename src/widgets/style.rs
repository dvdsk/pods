use iced::container;

pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 2] = [Theme::Light, Theme::Dark];
}

impl Default for Theme {
    fn default() -> Theme {
        Theme::Light
    }
}

impl From<Theme> for Box<dyn container::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => light::Container.into(),
            Theme::Dark => dark::Container.into(),
        }
    }
}

mod light {
    use iced::{container, Color};

    pub struct Container;

    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: Color::from_rgb8(0x36, 0x39, 0x3F).into(),
                text_color: Color::WHITE.into(),
                ..container::Style::default()
            }
        }
    }
}

mod dark {
    use iced::{container, Color};

    pub struct Container;

    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style::default()
        }
    }
}
