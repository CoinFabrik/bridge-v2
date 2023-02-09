use log::LevelFilter;
use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};

pub fn config(log_level: LevelFilter) {
    let pattern = Box::new(PatternEncoder::new(
        "[{d(%Y-%m-%d %H:%M:%S)} {l}] {M} — {m}{n}",
    ));

    let stdout = ConsoleAppender::builder().encoder(pattern.clone()).build();

    let _logfile = FileAppender::builder()
        .encoder(pattern)
        .build("log/output.log")
        .unwrap();

    let config = Config::builder()
        //.appender(Appender::builder().build("logfile", Box::new(logfile)))
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(log_level)))
                .build("stdout", Box::new(stdout)),
        )
        .build(
            Root::builder()
                .appender("stdout")
                //.appender("logfile")
                .build(LevelFilter::Trace),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}
