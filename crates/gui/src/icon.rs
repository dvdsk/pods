use iced::{
    alignment::{Horizontal, Vertical},
    widget, Font, Length,
};

const ICONS: Font = Font::External {
    name: "Icons",
    #[cfg(target_family = "unix")]
    bytes: include_bytes!(r"icon/MaterialIcons-Regular.ttf"),
    #[cfg(target_family = "windows")]
    bytes: include_bytes!(r"icon\MaterialIcons-Regular.ttf"),
};

fn icon(unicode: &'static str) -> widget::Text<'static> {
    widget::Text::new(unicode)
        .font(ICONS)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Left)
        .vertical_alignment(Vertical::Center)
}

macro_rules! icon_fn {
    ($($name:ident, $code:expr),+) => {
        $( // repeat for all input
            pub fn $name() -> widget::Text<'static> {
                icon($code)
            }
        )*
    }
}

icon_fn! {
    open_menu, "\u{e5d2}",
    close_menu, "\u{e9bd}"
}