//! Composer logic the panel and the coordinator share: the message queue,
//! image attachments with their caps, and `@` file mentions. No GTK here, so
//! it is unit-tested directly. `Packages/TexApp/Sources/AppPorts/AgentComposer.swift`
//! is the macOS twin; keep the two in step.

use std::sync::Arc;

/// Claude's own limits; the bridge refuses the same.
pub const MAX_IMAGES: usize = 8;
pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// One image the user attached: pi's `ImageContent` plus a name and size for the chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentImage {
    pub name: String,
    pub mime_type: String,
    /// Base64. Shared, because transcript entries and queue rows are cloned freely.
    pub data: Arc<str>,
    pub bytes: usize,
}

// The transcript hashes every row on each render; hashing megabytes of base64 there is waste.
impl std::hash::Hash for AgentImage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.bytes.hash(state);
        self.data.len().hash(state);
        self.data.get(..64).hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    TooLarge,
    TooMany,
    Unsupported,
}

impl ImageError {
    /// The `Localizable.strings` key the panel shows.
    pub fn key(self) -> &'static str {
        match self {
            Self::TooLarge => "assistant.image.too_large",
            Self::TooMany => "assistant.image.too_many",
            Self::Unsupported => "assistant.image.unsupported",
        }
    }
}

/// The image types Claude and Antigravity accept, by file extension.
pub fn image_mime(path: &str) -> Option<&'static str> {
    let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

impl AgentImage {
    /// Checks the type and the size cap; `attached` is how many the draft holds already.
    pub fn new(name: &str, mime_type: &str, bytes: &[u8], attached: usize) -> Result<Self, ImageError> {
        if !matches!(mime_type, "image/png" | "image/jpeg" | "image/gif" | "image/webp") {
            return Err(ImageError::Unsupported);
        }
        if attached >= MAX_IMAGES {
            return Err(ImageError::TooMany);
        }
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(ImageError::TooLarge);
        }
        Ok(Self {
            name: name.to_string(),
            mime_type: mime_type.to_string(),
            data: Arc::from(gtk4::glib::base64_encode(bytes).as_str()),
            bytes: bytes.len(),
        })
    }

    pub fn decoded(&self) -> Vec<u8> {
        gtk4::glib::base64_decode(&self.data)
    }

    /// `{type, mimeType, data}`: pi's ImageContent.
    pub fn wire(&self) -> serde_json::Value {
        serde_json::json!({"type": "image", "mimeType": self.mime_type, "data": &*self.data})
    }
}

/// "12 KB", "1.2 MB" for the chip.
pub fn byte_size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

// ─── Queue ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueuedState {
    Queued,
    /// Sent at agent_end; removed when the prompt is accepted, back to Queued if refused.
    Sending,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueuedMessage {
    pub id: u64,
    pub text: String,
    pub images: Vec<AgentImage>,
    pub state: QueuedState,
}

/// Messages typed while a turn runs (Paseo's queue): one is sent per agent_end.
#[derive(Debug, Default, Clone)]
pub struct MessageQueue {
    pub items: Vec<QueuedMessage>,
    /// Stop and a refused send hold the queue until the user sends again.
    pub paused: bool,
    next_id: u64,
}

/// What Return does with the draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitRoute {
    Send,
    Steer,
    Queue,
}

/// Return queues while a turn runs or an approval waits (Paseo sends nothing into a turn
/// that is waiting on the user). Cmd/Ctrl+Return steers when the backend can.
pub fn submit_route(running: bool, approval_pending: bool, now: bool, can_steer: bool) -> SubmitRoute {
    if !running && !approval_pending {
        SubmitRoute::Send
    } else if now && can_steer && !approval_pending {
        SubmitRoute::Steer
    } else {
        SubmitRoute::Queue
    }
}

impl MessageQueue {
    pub fn enqueue(&mut self, text: &str, images: Vec<AgentImage>) -> u64 {
        self.next_id += 1;
        self.items.push(QueuedMessage {
            id: self.next_id,
            text: text.to_string(),
            images,
            state: QueuedState::Queued,
        });
        self.next_id
    }

    /// The head to send at agent_end, marked Sending; None while paused or already sending.
    pub fn take_next(&mut self) -> Option<QueuedMessage> {
        if self.paused {
            return None;
        }
        let head = self.items.first_mut()?;
        if head.state == QueuedState::Sending {
            return None;
        }
        head.state = QueuedState::Sending;
        Some(head.clone())
    }

    /// The backend accepted it.
    pub fn sent(&mut self, id: u64) {
        self.items.retain(|item| item.id != id);
    }

    /// The backend refused it: back at the front, and the queue waits for the user.
    pub fn failed(&mut self, id: u64) {
        if let Some(index) = self.items.iter().position(|item| item.id == id) {
            let mut item = self.items.remove(index);
            item.state = QueuedState::Queued;
            self.items.insert(0, item);
            self.paused = true;
        }
    }

    /// Edit, Delete and Send now take the row out.
    pub fn remove(&mut self, id: u64) -> Option<QueuedMessage> {
        let index = self.items.iter().position(|item| item.id == id)?;
        Some(self.items.remove(index))
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.paused = false;
    }
}

// ─── @ mentions ─────────────────────────────────────────────────────────────

/// `@query` before the caret, in char offsets (GTK text iters count chars).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    pub start: usize,
    pub end: usize,
    pub query: String,
}

/// Paseo's rule: the `@` nearest the caret, not inside an email-like word,
/// with no whitespace or quote between it and the caret.
pub fn active_mention(text: &str, cursor: usize) -> Option<Mention> {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    let mut at = cursor;
    while at > 0 {
        at -= 1;
        let c = chars[at];
        if c == '@' {
            if at > 0 && (chars[at - 1].is_alphanumeric() || matches!(chars[at - 1], '_' | '+')) {
                // name@host: keep looking for an earlier @ (Paseo does the same).
                continue;
            }
            return Some(Mention {
                start: at,
                end: cursor,
                query: chars[at + 1..cursor].iter().collect(),
            });
        }
        if c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>') {
            return None;
        }
    }
    None
}

/// Fuzzy path match: every query char in order. File-name hits rank first, then
/// earlier and tighter matches, then shorter paths.
pub fn mention_matches<'a>(files: &'a [String], query: &str, limit: usize) -> Vec<&'a String> {
    let query = query.to_lowercase();
    let mut scored: Vec<(usize, &String)> = files
        .iter()
        .filter_map(|path| {
            let lower = path.to_lowercase();
            let name = lower.rsplit('/').next().unwrap_or(&lower);
            let score = if query.is_empty() {
                0
            } else if name.starts_with(&query) {
                0
            } else if name.contains(&query) {
                1
            } else if lower.contains(&query) {
                2
            } else {
                3 + subsequence_spread(&lower, &query)?
            };
            Some((score, path))
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.len().cmp(&b.1.len())).then(a.1.cmp(b.1)));
    scored.into_iter().take(limit).map(|(_, path)| path).collect()
}

/// How far apart the query's chars sit in `text`, or None when they are not all there in order.
fn subsequence_spread(text: &str, query: &str) -> Option<usize> {
    let mut chars = text.char_indices();
    let mut first = None;
    let mut last = 0;
    for q in query.chars() {
        let (index, _) = chars.find(|(_, c)| *c == q)?;
        first.get_or_insert(index);
        last = index;
    }
    Some(last - first.unwrap_or(0))
}

/// Replaces `@query` with `@path ` (quoted when the path has a space); returns the new
/// text and the caret after the inserted space, in chars.
pub fn apply_mention(text: &str, mention: &Mention, path: &str) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let token = if path.chars().any(char::is_whitespace) {
        format!("@\"{}\" ", path.replace('"', "\\\""))
    } else {
        format!("@{path} ")
    };
    let before: String = chars[..mention.start.min(chars.len())].iter().collect();
    let after: String = chars[mention.end.min(chars.len())..].iter().collect();
    let cursor = before.chars().count() + token.chars().count();
    (format!("{before}{token}{after}"), cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(attached: usize) -> Result<AgentImage, ImageError> {
        AgentImage::new("a.png", "image/png", &[1, 2, 3], attached)
    }

    #[test]
    fn image_caps_and_types() {
        let image = png(0).unwrap();
        assert_eq!(image.bytes, 3);
        assert_eq!(image.decoded(), vec![1, 2, 3]);
        assert_eq!(image.wire()["mimeType"], "image/png");
        assert_eq!(image.wire()["type"], "image");
        assert_eq!(png(MAX_IMAGES).unwrap_err(), ImageError::TooMany);
        assert!(png(MAX_IMAGES - 1).is_ok());
        let big = vec![0u8; MAX_IMAGE_BYTES + 1];
        assert_eq!(AgentImage::new("b.png", "image/png", &big, 0).unwrap_err(), ImageError::TooLarge);
        assert!(AgentImage::new("b.png", "image/png", &big[1..], 0).is_ok());
        assert_eq!(AgentImage::new("c.tiff", "image/tiff", &[1], 0).unwrap_err(), ImageError::Unsupported);
        assert_eq!(image_mime("shots/Red.PNG"), Some("image/png"));
        assert_eq!(image_mime("a.jpeg"), Some("image/jpeg"));
        assert_eq!(image_mime("main.tex"), None);
        assert_eq!(byte_size(1500), "2 KB");
        assert_eq!(byte_size(3 * 1024 * 1024 / 2), "1.5 MB");
    }

    #[test]
    fn return_queues_during_a_turn_and_while_an_approval_waits() {
        assert_eq!(submit_route(false, false, false, true), SubmitRoute::Send);
        assert_eq!(submit_route(false, false, true, true), SubmitRoute::Send);
        assert_eq!(submit_route(true, false, false, true), SubmitRoute::Queue);
        assert_eq!(submit_route(true, false, true, true), SubmitRoute::Steer);
        assert_eq!(submit_route(true, false, true, false), SubmitRoute::Queue, "no steer: Ctrl+Return queues");
        assert_eq!(submit_route(true, true, true, true), SubmitRoute::Queue, "approval pending: never into the turn");
    }

    #[test]
    fn queue_drains_one_per_turn_and_a_refused_send_goes_back_to_the_front() {
        let mut queue = MessageQueue::default();
        let a = queue.enqueue("first", vec![]);
        let b = queue.enqueue("second", vec![png(0).unwrap()]);
        let head = queue.take_next().unwrap();
        assert_eq!((head.id, head.state), (a, QueuedState::Sending));
        assert!(queue.take_next().is_none(), "one in flight at a time");
        queue.failed(a);
        assert_eq!(queue.items[0].id, a);
        assert_eq!(queue.items[0].state, QueuedState::Queued);
        assert!(queue.paused);
        assert!(queue.take_next().is_none(), "paused after a refusal");
        queue.paused = false;
        assert_eq!(queue.take_next().unwrap().id, a);
        queue.sent(a);
        let next = queue.take_next().unwrap();
        assert_eq!((next.id, next.images.len()), (b, 1));
        queue.sent(b);
        assert!(queue.items.is_empty());
    }

    #[test]
    fn edit_delete_and_send_now_take_the_row_out() {
        let mut queue = MessageQueue::default();
        let a = queue.enqueue("one", vec![]);
        let b = queue.enqueue("two", vec![]);
        let c = queue.enqueue("three", vec![]);
        assert_eq!(queue.remove(b).unwrap().text, "two");
        assert!(queue.remove(b).is_none());
        assert_eq!(queue.items.iter().map(|i| i.id).collect::<Vec<_>>(), vec![a, c]);
        queue.paused = true;
        queue.clear();
        assert!(queue.items.is_empty() && !queue.paused);
    }

    #[test]
    fn mention_trigger_skips_emails_and_stops_at_whitespace() {
        assert_eq!(
            active_mention("see @ch", 7),
            Some(Mention { start: 4, end: 7, query: "ch".into() })
        );
        assert_eq!(active_mention("@", 1).unwrap().query, "");
        assert_eq!(active_mention("mail me@host.com", 16), None);
        assert_eq!(active_mention("@a b", 4), None);
        assert_eq!(active_mention("@\"x", 3), None);
        // The caret, not the end of the text, decides.
        assert_eq!(active_mention("@intro and more", 6).unwrap().query, "intro");
        assert_eq!(active_mention("한글 @서론", 6).unwrap().query, "서론");
    }

    #[test]
    fn mention_filter_is_fuzzy_and_ranks_file_names_first() {
        let files: Vec<String> = ["chapters/intro.tex", "main.tex", "figures/plot intro.png", "refs.bib", "chapters/method.tex"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(mention_matches(&files, "intro", 8), vec!["chapters/intro.tex", "figures/plot intro.png"]);
        assert_eq!(mention_matches(&files, "chm", 8), vec!["chapters/method.tex"]);
        assert_eq!(mention_matches(&files, "MAIN", 8), vec!["main.tex"]);
        assert_eq!(mention_matches(&files, "", 2).len(), 2);
        assert!(mention_matches(&files, "zzz", 8).is_empty());
    }

    #[test]
    fn mention_insert_quotes_paths_with_spaces() {
        let text = "look at @int please";
        let mention = active_mention(text, 12).unwrap();
        assert_eq!(apply_mention(text, &mention, "chapters/intro.tex"), ("look at @chapters/intro.tex  please".into(), 28));
        let (quoted, cursor) = apply_mention("@plot", &active_mention("@plot", 5).unwrap(), "figures/plot intro.png");
        assert_eq!(quoted, "@\"figures/plot intro.png\" ");
        assert_eq!(cursor, quoted.chars().count());
    }
}

