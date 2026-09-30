//! Line-protocol adapter; executes the production scheduler, not a policy copy.
#[allow(dead_code)]
#[path = "../Linux/crates/build-feature/src/live.rs"]
mod live;

use live::{LiveCompileScheduler, LiveCompletion, LiveRequest, LiveRunToken};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

fn main() {
    let mut scheduler = LiveCompileScheduler::new();
    let mut tokens: HashMap<u64, LiveRunToken> = HashMap::new();
    let mut out = io::BufWriter::new(io::stdout());
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let args: Vec<_> = line.split_whitespace().collect();
        let number = |i: usize| args[i].parse::<u64>().unwrap();
        let mut status = "-";
        let requests = match args[0] {
            "reset" => {
                scheduler = LiveCompileScheduler::new();
                tokens.clear();
                scheduler.set_delay(number(1));
                scheduler.set_enabled(true)
            }
            "edit" => scheduler.note_edit(number(1)),
            "poll" => scheduler.poll(number(1), number(2) != 0),
            "manual" => scheduler.request_manual(),
            "enable" => scheduler.set_enabled(number(1) != 0),
            "invalidate" => scheduler.invalidate(),
            "finish" => {
                let (completion, requests) = scheduler.completed(
                    tokens[&number(1)], number(2), number(3) != 0,
                );
                status = match completion {
                    LiveCompletion::Current(_) => "current",
                    LiveCompletion::Superseded(_) => "superseded",
                    LiveCompletion::Stale => "stale",
                };
                requests
            }
            _ => panic!("unknown command: {line}"),
        };
        let requests: Vec<_> = requests.into_iter().map(|request| {
            let (kind, token) = match request {
                LiveRequest::StartLive { token, .. } => ("live", token),
                LiveRequest::StartManual { token } => ("manual", token),
                LiveRequest::Cancel { token } => ("cancel", token),
            };
            tokens.insert(token.id, token);
            format!("{kind}:{}", token.id)
        }).collect();
        let optional = |value: Option<u64>| value.map_or("-".into(), |n| n.to_string());
        let active = scheduler.active();
        writeln!(out, "{}|{}|{}|{}|{}", optional(scheduler.pending_deadline()),
            optional(active.map(|t| t.id)),
            u8::from(active.map_or(false, |t| scheduler.accepts_active_result(t))),
            status, requests.join(",")).unwrap();
        out.flush().unwrap();
    }
}
