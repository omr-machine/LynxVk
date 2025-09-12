use simplelog::SimpleLogger;

mod mess;

fn main() {
    let _ = SimpleLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default());
    mess::ash_test_main::ash_test_main();
}
