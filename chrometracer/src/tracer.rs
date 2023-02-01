use crossbeam_channel::Sender;
use crossbeam_queue::ArrayQueue;
use derive_builder::Builder;
use std::{
    cell::RefCell,
    fs::File,
    io,
    thread::{self, JoinHandle},
    time::SystemTime,
};
use tracing_chrometrace::{ChromeEvent, ChromeEventBuilder, EventType};

thread_local! {
    static CURRENT: RefCell<Option<ChromeTracer>> = RefCell::new(None);
}

static mut GLOBAL: Option<ChromeTracer> = None;

#[derive(Builder, Clone)]
#[builder(custom_constructor, build_fn(private, name = "_build"))]
pub struct ChromeTracer {
    #[builder(default = "SystemTime::now()")]
    pub start: SystemTime,

    #[builder(setter(skip))]
    sender: Option<Sender<ChromeTracerMessage>>,
}

#[allow(clippy::large_enum_variant)]
enum ChromeTracerMessage {
    ChromeEvent(ChromeEvent),
    Terminate,
}

pub struct ChromeTracerGuard {
    sender: Sender<ChromeTracerMessage>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for ChromeTracerGuard {
    fn drop(&mut self) {
        self.sender.send(ChromeTracerMessage::Terminate).unwrap();
        self.handle.take().map(JoinHandle::join).unwrap().unwrap();
    }
}

impl ChromeTracerBuilder {
    pub fn init(&self) -> ChromeTracerGuard {
        CURRENT.with(|c| {
            if unsafe { GLOBAL.is_some() } {
                panic!("Unable to intialize ChromeTracer. A chrometracer already been set");
            } else {
                let mut tracer = self._build().expect("All required fields were initialized");
                let guard = tracer.init();

                unsafe { GLOBAL = Some(tracer.clone()) };
                *c.borrow_mut() = Some(tracer);

                guard
            }
        })
    }
}

pub fn builder() -> ChromeTracerBuilder {
    ChromeTracerBuilder::create_empty()
}

impl ChromeTracer {
    fn init(&mut self) -> ChromeTracerGuard {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.sender = Some(sender.clone());

        let handle = Some(thread::spawn(move || {
            let mut file = File::create("trace.json").unwrap();
            let queue = ArrayQueue::new(1);

            io::Write::write_all(&mut file, b"[\n").unwrap();

            while let Ok(ChromeTracerMessage::ChromeEvent(event)) = receiver.recv() {
                let s = serde_json::to_string(&event).unwrap();
                if let Some(e) = queue.force_push(s) {
                    io::Write::write_all(&mut file, e.as_bytes()).unwrap();
                    io::Write::write_all(&mut file, b",\n").unwrap();
                };
            }

            if let Some(e) = queue.pop() {
                io::Write::write_all(&mut file, e.as_bytes()).unwrap();
                io::Write::write_all(&mut file, b"\n").unwrap();
            }

            io::Write::write_all(&mut file, b"]").unwrap();
        }));

        ChromeTracerGuard { sender, handle }
    }

    pub fn trace(&self, event: ChromeEvent) {
        let _ = self
            .sender
            .as_ref()
            .map(|sender| sender.send(ChromeTracerMessage::ChromeEvent(event)));
    }
}

pub fn current<T, F>(mut f: F) -> T
where
    F: FnMut(Option<&ChromeTracer>) -> T,
{
    CURRENT.with(|c| {
        let mut tracer = c.borrow_mut();
        if tracer.is_none() {
            *tracer = unsafe { GLOBAL.clone() };
        }

        f(tracer.as_ref())
    })
}

#[macro_export]
macro_rules! event {
    ($($key:ident = $value:expr),*) => {

        $crate::current(|tracer| {
            if let Some(tracer) = tracer {
                use $crate::Recordable as _;

                let mut builder = $crate::ChromeEvent::builder(tracer.start);

                $(
                    $value.record(&mut builder, stringify!($key));
                )*

                let event = builder.build().unwrap();
                tracer.trace(event);
            }
        })
    };
}

pub trait Recordable {
    type Item;

    fn record(self, builder: &mut ChromeEventBuilder, name: &'static str);
}

impl Recordable for u64 {
    type Item = u64;

    fn record(self, builder: &mut ChromeEventBuilder, name: &'static str) {
        match name {
            "tid" => builder.tid(self),
            "pid" => builder.pid(self),
            _ => builder.arg((name.to_string(), self.to_string())),
        };
    }
}

impl Recordable for &'static str {
    type Item = &'static str;

    fn record(self, builder: &mut ChromeEventBuilder, name: &'static str) {
        match name {
            "name" => builder.name(self),
            "cat" => builder.cat(self),
            "id" => builder.id(self),
            _ => builder.arg((name.to_string(), self.to_string())),
        };
    }
}

impl Recordable for String {
    type Item = String;

    fn record(self, builder: &mut ChromeEventBuilder, name: &'static str) {
        match name {
            "name" => builder.name(self),
            "cat" => builder.cat(self),
            "id" => builder.id(self),
            _ => builder.arg((name.to_string(), self)),
        };
    }
}

impl Recordable for f64 {
    type Item = f64;

    fn record(self, builder: &mut ChromeEventBuilder, name: &'static str) {
        match name {
            "ts" => builder.ts(self),
            "dur" => builder.dur(Some(self)),
            "tts" => builder.tts(Some(self)),
            _ => builder.arg((name.to_string(), self.to_string())),
        };
    }
}

impl Recordable for EventType {
    type Item = EventType;

    fn record(self, builder: &mut ChromeEventBuilder, name: &'static str) {
        match name {
            "ph" => builder.ph(self),
            _ => builder.arg((name.to_string(), self.as_ref().to_string())),
        };
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn event() {
        crate::builder().init();

        event!(name = "hello");
    }

    #[test]
    fn without_init() {
        event!(name = "hello");
    }
}
