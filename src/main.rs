use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["healthcheck"] {
        return match ocstats::check_default_database() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("ocstats: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if let [command] | [command, _] | [command, _, _] = arguments.as_slice()
        && command == "serve"
    {
        let port = match arguments.as_slice() {
            [_] => match std::env::var("OCSTATS_PORT") {
                Ok(port) => match port.parse() {
                    Ok(port) => port,
                    Err(_) => return usage(),
                },
                Err(std::env::VarError::NotPresent) => 4117,
                Err(std::env::VarError::NotUnicode(_)) => return usage(),
            },
            [_, port] => match port.parse() {
                Ok(port) => port,
                Err(_) => return usage(),
            },
            [_, flag, port] if flag == "--port" => match port.parse() {
                Ok(port) => port,
                Err(_) => return usage(),
            },
            _ => return usage(),
        };
        return match ocstats::serve_default(port).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("ocstats: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if !arguments.is_empty() {
        return usage();
    }
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

fn usage() -> ExitCode {
    eprintln!("usage: ocstats [healthcheck|serve [--port PORT|PORT]]");
    ExitCode::FAILURE
}
