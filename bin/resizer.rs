use lambda_runtime::{error::HandlerError, lambda, Context};
use log::LevelFilter;
use resizer::{run, InputEvent};
use simple_logger::SimpleLogger;
use std::error::Error as StdError;

fn resize(event: InputEvent, _context: Context) -> Result<(), HandlerError> {
    Ok(run(event))
}

fn main() -> Result<(), Box<dyn StdError>> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .without_timestamps()
        .init()
        .unwrap();
    lambda!(resize);
    Ok(())
}
