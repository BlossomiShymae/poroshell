use std::{
    fs::{self, OpenOptions},
    path::Path,
};

use time::{OffsetDateTime, format_description};
use tracing::Level;
use tracing_subscriber::{Layer, Registry, filter, fmt, layer::SubscriberExt};

pub fn setup() {
    let format_description = format_description::parse("[year]-[month]-[day]").unwrap();
    let now = OffsetDateTime::now_utc()
        .format(&format_description)
        .unwrap();

    let _ = fs::create_dir_all("logs");
    let debug_filename = format!("logs/debug-{now}.log");
    let debug_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(Path::new(&debug_filename))
        .unwrap();

    let debug_file_layer = fmt::layer()
        .with_ansi(false)
        .pretty()
        .with_writer(debug_file)
        .with_filter(filter::LevelFilter::from_level(Level::DEBUG));

    let subscriber = Registry::default().with(debug_file_layer);
    let _ = tracing::subscriber::set_global_default(subscriber);
}
