use std::process::ExitCode;

fn main() -> ExitCode {
    match ocstats::extract_default() {
        Ok(data) => {
            let mut store = match ocstats::AnalyticsStore::open_default() {
                Ok(store) => store,
                Err(error) => {
                    eprintln!("ocstats: {error}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(error) = store.import(&data) {
                eprintln!("ocstats: {error}");
                return ExitCode::FAILURE;
            }
            println!(
                "Imported {} sessions, {} assistant messages, and {} completed steps.",
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
