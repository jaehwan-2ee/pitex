//! How the transcript is drawn (Paseo `collapseTurns` and subagent grouping): a subagent's
//! calls nest under its Task/Agent card, and a finished turn folds its work and earlier
//! replies into one summary row under the prompt, leaving the final reply in view.

use crate::agent::{AgentTranscriptEntry, EntryMeta, TranscriptRole};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViewItem {
    /// One transcript row; a subagent card carries the rows of the calls it made.
    Row { index: usize, children: Vec<usize> },
    /// A finished turn's work, folded under its prompt (row `prompt`).
    Summary { prompt: usize, summary: TurnSummary, rows: Vec<ViewItem> },
}

/// "Worked for 1m 12s · 3 edits · 2 commands · 4 reads" (Paseo's overview counts).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TurnSummary {
    pub seconds: Option<u64>,
    /// Files edited or written (each file once).
    pub edits: usize,
    pub commands: usize,
    /// Files read (each file once).
    pub reads: usize,
    /// Searches and fetches.
    pub searches: usize,
    /// Every other call (subagents, MCP and pi tools).
    pub tools: usize,
    pub thoughts: usize,
}

impl TurnSummary {
    /// The non-zero counts in display order, as (`assistant.turn.<kind>` key stem, count).
    pub fn parts(&self) -> Vec<(&'static str, usize)> {
        [
            ("edits", self.edits),
            ("commands", self.commands),
            ("reads", self.reads),
            ("searches", self.searches),
            ("tools", self.tools),
            ("thoughts", self.thoughts),
        ]
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .collect()
    }
}

/// The rows to draw for `entries`; the running turn (the last, while `running`) stays open.
pub fn view_items(entries: &[AgentTranscriptEntry], running: bool) -> Vec<ViewItem> {
    let children = subagent_children(entries);
    let nested: HashSet<usize> = children.values().flatten().copied().collect();
    let row = |index: usize| ViewItem::Row { index, children: children.get(&index).cloned().unwrap_or_default() };
    let top: Vec<usize> = (0..entries.len()).filter(|i| !nested.contains(i)).collect();

    let mut items = Vec::new();
    let mut start = 0;
    while start < top.len() {
        // A turn: a prompt and everything up to the next prompt (rows before the first prompt stand alone).
        let mut end = start + 1;
        while end < top.len() && entries[top[end]].role != TranscriptRole::User {
            end += 1;
        }
        let turn = &top[start..end];
        let prompt = turn[0];
        let last_turn = end == top.len();
        let reply = turn.iter().rposition(|&i| entries[i].role == TranscriptRole::Assistant);
        let foldable = entries[prompt].role == TranscriptRole::User && !(running && last_turn);
        let folded: Vec<usize> = match reply {
            Some(reply) if foldable => turn[1..reply].iter().copied().filter(|&i| entries[i].role != TranscriptRole::Notice).collect(),
            _ => Vec::new(),
        };
        // Paseo folds only a turn that worked or answered more than once.
        let worked = folded.iter().any(|&i| matches!(entries[i].role, TranscriptRole::Tool | TranscriptRole::Thinking | TranscriptRole::Assistant));
        if !worked {
            items.extend(turn.iter().map(|&i| row(i)));
        } else {
            items.push(row(prompt));
            let seconds = match entries[prompt].meta {
                EntryMeta::Worked { seconds } => Some(seconds),
                _ => None,
            };
            items.push(ViewItem::Summary {
                prompt,
                summary: summarize(entries, &folded, seconds),
                rows: folded.iter().map(|&i| row(i)).collect(),
            });
            // Notices (errors, compaction) stay in view, then the final reply and anything after it.
            let reply = reply.unwrap_or(turn.len());
            items.extend(turn[1..].iter().enumerate().filter(|(n, &i)| *n + 1 >= reply || entries[i].role == TranscriptRole::Notice).map(|(_, &i)| row(i)));
        }
        start = end;
    }
    items
}

/// Subagent card row → the rows of the calls made inside it (nested subagents flatten into the outermost card).
fn subagent_children(entries: &[AgentTranscriptEntry]) -> HashMap<usize, Vec<usize>> {
    let mut card_of: HashMap<&str, usize> = HashMap::new();
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(tool) = &entry.tool else { continue };
        let parent = tool.parent_call_id.as_deref().and_then(|id| card_of.get(id).copied());
        if let Some(card) = parent {
            children.entry(card).or_default().push(index);
        }
        if tool.kind == "sub_agent" && !tool.call_id.is_empty() {
            card_of.insert(tool.call_id.as_str(), parent.unwrap_or(index));
        } else if let (Some(card), false) = (parent, tool.call_id.is_empty()) {
            card_of.entry(tool.call_id.as_str()).or_insert(card);
        }
    }
    children
}

fn summarize(entries: &[AgentTranscriptEntry], rows: &[usize], seconds: Option<u64>) -> TurnSummary {
    let mut summary = TurnSummary { seconds, ..Default::default() };
    let mut edited = HashSet::new();
    let mut read = HashSet::new();
    for &index in rows {
        let entry = &entries[index];
        if entry.role == TranscriptRole::Thinking {
            summary.thoughts += 1;
        }
        if entry.role != TranscriptRole::Tool {
            continue;
        }
        let tool = entry.tool.as_ref();
        let path = tool.and_then(|t| t.path.clone()).unwrap_or_else(|| format!("#{index}"));
        match tool.map(|t| t.kind.as_str()).unwrap_or("other") {
            "edit" | "write" => {
                edited.insert(path);
            }
            "read" => {
                read.insert(path);
            }
            "shell" => summary.commands += 1,
            "search" | "fetch" => summary.searches += 1,
            _ => summary.tools += 1,
        }
    }
    summary.edits = edited.len();
    summary.reads = read.len();
    summary
}

/// "1m 12s", "45s", "<1s".
pub fn duration_text(lang: &str, seconds: u64) -> String {
    match seconds {
        0 => crate::l10n::tr1(lang, "assistant.turn.seconds", "<1"),
        s if s < 60 => crate::l10n::tr1(lang, "assistant.turn.seconds", &s.to_string()),
        s => crate::l10n::trn(lang, "assistant.turn.minutes", &[&(s / 60).to_string(), &(s % 60).to_string()]),
    }
}

/// The summary row's one line.
pub fn summary_text(lang: &str, summary: &TurnSummary) -> String {
    let mut parts = vec![match summary.seconds {
        Some(seconds) => crate::l10n::tr1(lang, "assistant.turn.worked_for", &duration_text(lang, seconds)),
        None => crate::l10n::tr(lang, "assistant.turn.worked"),
    }];
    for (kind, count) in summary.parts() {
        parts.push(if count == 1 {
            crate::l10n::tr(lang, &format!("assistant.turn.{kind}.one"))
        } else {
            crate::l10n::tr1(lang, &format!("assistant.turn.{kind}.other"), &count.to_string())
        });
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{ToolDetail, TranscriptStatus};

    fn entry(role: TranscriptRole, text: &str) -> AgentTranscriptEntry {
        AgentTranscriptEntry {
            role,
            title: String::new(),
            text: text.into(),
            detail: String::new(),
            status: TranscriptStatus::Done,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::None,
        }
    }

    fn tool(id: &str, kind: &str, path: Option<&str>, parent: Option<&str>) -> AgentTranscriptEntry {
        let mut row = entry(TranscriptRole::Tool, "");
        row.tool = Some(ToolDetail {
            call_id: id.into(),
            kind: kind.into(),
            path: path.map(String::from),
            parent_call_id: parent.map(String::from),
            ..Default::default()
        });
        row
    }

    fn prompt(text: &str, seconds: Option<u64>) -> AgentTranscriptEntry {
        let mut row = entry(TranscriptRole::User, text);
        if let Some(seconds) = seconds {
            row.meta = EntryMeta::Worked { seconds };
        }
        row
    }

    #[test]
    fn a_finished_turn_folds_its_work_under_the_prompt_and_keeps_the_final_reply() {
        let rows = vec![
            prompt("Fix it", Some(72)),
            entry(TranscriptRole::Thinking, "plan"),
            entry(TranscriptRole::Assistant, "Reading first."),
            tool("r1", "read", Some("a.tex"), None),
            tool("r2", "read", Some("a.tex"), None),
            tool("e1", "edit", Some("a.tex"), None),
            tool("e2", "write", Some("b.tex"), None),
            tool("s1", "shell", None, None),
            tool("g1", "search", None, None),
            entry(TranscriptRole::Notice, "Compacting the conversation…"),
            entry(TranscriptRole::Assistant, "Done."),
        ];
        let items = view_items(&rows, false);
        let row = |index| ViewItem::Row { index, children: vec![] };
        let ViewItem::Summary { prompt, summary, rows: folded } = &items[1] else { panic!("no summary: {items:?}") };
        assert_eq!(*prompt, 0);
        assert_eq!(folded, &(1..9).map(row).collect::<Vec<_>>());
        assert_eq!(
            summary,
            &TurnSummary { seconds: Some(72), edits: 2, commands: 1, reads: 1, searches: 1, tools: 0, thoughts: 1 }
        );
        assert_eq!(items[0], row(0));
        assert_eq!(&items[2..], &[row(9), row(10)], "the notice and the final reply stay in view");
        assert_eq!(summary_text("en", summary), "Worked for 1m 12s · 2 edits · 1 command · 1 read · 1 search · 1 thought");
    }

    #[test]
    fn the_running_turn_and_turns_without_work_stay_open() {
        let rows = vec![
            prompt("Hi", None),
            entry(TranscriptRole::Assistant, "Hello."),
            prompt("Build", None),
            tool("s1", "shell", None, None),
            entry(TranscriptRole::Assistant, "Building…"),
        ];
        let open: Vec<ViewItem> = (0..5).map(|index| ViewItem::Row { index, children: vec![] }).collect();
        assert_eq!(view_items(&rows, true), open, "a plain answer never folds; the running turn stays open");
        let done = view_items(&rows, false);
        assert!(matches!(&done[3], ViewItem::Summary { prompt: 2, summary, .. } if summary.commands == 1 && summary.seconds.is_none()));
        assert_eq!(summary_text("en", &TurnSummary { commands: 1, ..Default::default() }), "Worked · 1 command");
        // A turn with tools but no reply (failed, stopped) does not fold either.
        assert_eq!(view_items(&rows[..4], false).len(), 4);
    }

    #[test]
    fn subagent_calls_nest_under_their_card_and_are_not_counted_twice() {
        let mut agent = tool("a", "sub_agent", None, None);
        agent.status = TranscriptStatus::Done;
        let rows = vec![
            prompt("Look it up", Some(5)),
            agent,
            tool("g", "search", None, Some("a")),
            tool("inner", "sub_agent", None, Some("a")),
            tool("r", "read", Some("x"), Some("inner")),
            tool("r2", "read", Some("y"), None),
            entry(TranscriptRole::Assistant, "beta"),
        ];
        let items = view_items(&rows, false);
        let ViewItem::Summary { rows: folded, summary, .. } = &items[1] else { panic!("{items:?}") };
        assert_eq!(folded, &vec![ViewItem::Row { index: 1, children: vec![2, 3, 4] }, ViewItem::Row { index: 5, children: vec![] }]);
        assert_eq!((summary.tools, summary.reads, summary.searches), (1, 1, 0));
        // While the turn runs the card is a plain row with its children.
        let live = view_items(&rows[..3], true);
        assert_eq!(live[1], ViewItem::Row { index: 1, children: vec![2] });
        assert_eq!(live.len(), 2);
    }

    #[test]
    fn durations_read_like_the_thinking_rows() {
        assert_eq!(duration_text("en", 0), "<1s");
        assert_eq!(duration_text("en", 45), "45s");
        assert_eq!(duration_text("en", 125), "2m 5s");
    }
}
