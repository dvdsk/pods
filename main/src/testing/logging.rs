pub fn set_error_hook() {
    use std::sync::Once;
    static COLOR_EYRE_SETUP: Once = Once::new();
    COLOR_EYRE_SETUP.call_once(|| {
        color_eyre::config::HookBuilder::default()
            // .issue_filter(filter) // TODO
            // .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
            // .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
            // .capture_span_trace_by_default(true)
            .install()
            .expect("could not set up error reporting")
    });
}

pub fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    use std::sync::Once;
    static TRACING_SETUP: Once = Once::new();
    TRACING_SETUP.call_once(|| {
        let fmt_layer = fmt::layer().pretty().with_line_number(true);
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("info"))
            .unwrap()
            .add_directive("panda=debug".parse().unwrap())
            .add_directive("wgpu_core=warn".parse().unwrap())
            .add_directive("wgpu_hal=error".parse().unwrap())
            .add_directive("iced_wgpu=warn".parse().unwrap())
            .add_directive("iced_winit=warn".parse().unwrap());

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .with(ErrorLayer::default())
            .init();
    });
}
