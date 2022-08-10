mod tracer;

pub use chrometracer_attributes::instrument;
pub use tracer::{builder, current, Recordable};
pub use tracing_chrometrace::ChromeEvent;
pub use tracing_chrometrace::EventType;
