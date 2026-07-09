use std::process::ExitCode;

fn main() -> ExitCode {
    match ocstats::extract_default() {
        Ok(data) => {
            println!(
                "Extracted {} sessions, {} assistant messages, and {} completed steps.",
                data.sessions.len(),
                data.assistant_messages.len(),
                data.steps.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("ocstats: {error}");
            ExitCode::FAILURE
        }
    }
}
