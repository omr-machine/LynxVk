use log::SetLoggerError;
// use simplelog::SimpleLogger;

use mess;
use teapot;

fn main() {
    let _ = setup_logger();

    // mess::ash_test_main();
    teapot::main();
}

fn setup_logger() -> std::result::Result<(), SetLoggerError> {
    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![simplelog::TermLogger::new(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )];
    if let Ok(file) = std::fs::File::create("log.txt") {
        loggers.push(simplelog::WriteLogger::new(
            simplelog::LevelFilter::Trace,
            simplelog::Config::default(),
            file,
        ));
    }
    let simple_logger = simplelog::CombinedLogger::init(loggers);

    // let simple_logger = SimpleLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default());

    return simple_logger;
}
