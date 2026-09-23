// Shared Markdown renderer + preview glue, inlined into
// Mac/Resources/markdown-preview.html by Tools/build-markdown-preview.mjs.
// renderMarkdown() is DOM-free so node can test it under vm; the window.*
// entry points are what the native web views call.

const pitexMd = markdownit({ html: true, linkify: true });

pitexMd.use(texmath, {
    engine: katex,
    delimiters: ['dollars', 'brackets'],
    katexOptions: { throwOnError: false }
});

// texmath's inline rules stop at a line break, and it treats `\[…\]` as
// display math only when it stands in a block of its own — so math that
// wraps across the soft line breaks of one paragraph stayed plain text.
// This rule runs after texmath's (both sit before 'escape') and matches
// those across lines. `$` follows pandoc: no space just inside the
// delimiters and no digit right after, so "$5 and $10" stays text.
const PITEX_WRAPPED_MATH = [['\\[', '\\]', true], ['\\(', '\\)', false], ['$', '$', false]];

pitexMd.inline.ruler.before('escape', 'pitex_wrapped_math', (state, silent) => {
    for (const [open, close, display] of PITEX_WRAPPED_MATH) {
        if (!state.src.startsWith(open, state.pos)) continue;
        const start = state.pos + open.length;
        const end = state.src.indexOf(close, start);
        if (end <= start) continue;
        const content = state.src.slice(start, end);
        if (open === '$' && (state.src[start] === '$' || /^\s|\s$/.test(content) ||
            /\d/.test(state.src[end + 1] ?? ''))) continue;
        if (!silent) {
            const token = state.push('pitex_math', 'math', 0);
            token.content = content;
            token.meta = { display };
        }
        state.pos = end + close.length;
        return true;
    }
    return false;
});

pitexMd.renderer.rules.pitex_math = (tokens, idx) => katex.renderToString(tokens[idx].content, {
    displayMode: tokens[idx].meta.display, throwOnError: false
});

// Block tokens rendered through renderToken/renderAttrs get data-line via a
// core pass; the rules that bypass it (fence, html_block, texmath's sections)
// are wrapped below to inject the same attribute.
const PITEX_LINE_TYPES = new Set([
    'paragraph_open', 'heading_open', 'blockquote_open', 'list_item_open',
    'table_open', 'hr', 'code_block'
]);

pitexMd.core.ruler.push('pitex_lines', state => {
    const walk = tokens => {
        for (const token of tokens) {
            if (token.map && PITEX_LINE_TYPES.has(token.type))
                token.attrSet('data-line', String(token.map[0]));
            if (token.children) walk(token.children);
        }
    };
    walk(state.tokens);
});

const pitexLineAttr = token => token.map ? ` data-line="${token.map[0]}"` : '';

const pitexDefaultFence = pitexMd.renderer.rules.fence;
pitexMd.renderer.rules.fence = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    // ```math fences are KaTeX display math, like texmath's block rules.
    if (token.info.trim().split(/\s+/)[0] === 'math')
        return `<section${pitexLineAttr(token)}><eqn>${
            katex.renderToString(token.content, { displayMode: true, throwOnError: false })
        }</eqn></section>\n`;
    const html = pitexDefaultFence(tokens, idx, options, env, self);
    return token.map ? html.replace('<pre', `<pre${pitexLineAttr(token)}`) : html;
};

const pitexHtmlBlock = pitexMd.renderer.rules.html_block;
pitexMd.renderer.rules.html_block = (tokens, idx, options, env, self) =>
    `<div${pitexLineAttr(tokens[idx])}>` +
    pitexHtmlBlock(tokens, idx, options, env, self) + '</div>';

for (const name of ['math_block', 'math_block_eqno']) {
    const inner = pitexMd.renderer.rules[name];
    pitexMd.renderer.rules[name] = (tokens, idx, options, env, self) =>
        inner(tokens, idx, options, env, self)
            .replace('<section', `<section${pitexLineAttr(tokens[idx])}`);
}

function renderMarkdown(text) {
    return pitexMd.render(String(text ?? ''));
}

// Pixels to keep the native sides quiet for after a programmatic scroll or a
// re-render, so we don't bounce scroll positions back and forth.
const PITEX_SCROLL_SUPPRESS_MS = 120;
let pitexSuppressUntil = 0;
let pitexScrollTick = false;

// [source line, document Y] pairs for every marked block element.
function pitexLineMap() {
    return Array.from(document.querySelectorAll('[data-line]')).map(el => [
        Number(el.getAttribute('data-line')),
        el.getBoundingClientRect().top + window.scrollY
    ]);
}

function pitexRender(opts) {
    const root = document.documentElement;
    const top = window.scrollY;
    document.getElementById('preview-base').setAttribute('href', opts.baseHref || 'file:///');
    if (opts.fontSize) root.style.fontSize = opts.fontSize + 'px';
    root.classList.toggle('dark', !!opts.dark);
    document.getElementById('content').innerHTML = renderMarkdown(opts.text);
    pitexSuppressUntil = Date.now() + PITEX_SCROLL_SUPPRESS_MS;
    window.scrollTo(0, top);
}

function pitexScrollToLine(line) {
    const marks = pitexLineMap();
    if (!marks.length) return;
    let prev = null, next = null;
    for (const mark of marks) {
        if (mark[0] <= line) prev = mark;
        else { next = mark; break; }
    }
    const y = !prev ? 0
        : !next || next[0] === prev[0] ? prev[1]
        : prev[1] + (line - prev[0]) / (next[0] - prev[0]) * (next[1] - prev[1]);
    pitexSuppressUntil = Date.now() + PITEX_SCROLL_SUPPRESS_MS;
    window.scrollTo(0, Math.max(0, y));
}

function pitexTopLine() {
    const marks = pitexLineMap();
    if (!marks.length) return 0;
    const y = window.scrollY;
    if (y <= marks[0][1]) return marks[0][0];
    let prev = marks[0], next = null;
    for (const mark of marks) {
        if (mark[1] <= y) prev = mark;
        else { next = mark; break; }
    }
    if (!next || next[0] === prev[0]) return prev[0];
    return Math.round(prev[0] + (y - prev[1]) / (next[1] - prev[1]) * (next[0] - prev[0]));
}

if (typeof window !== 'undefined') {
    window.pitexRender = pitexRender;
    window.pitexScrollToLine = pitexScrollToLine;

    // Same message-handler API in WKWebView and WebKitGTK.
    window.addEventListener('scroll', () => {
        if (pitexScrollTick) return;
        pitexScrollTick = true;
        requestAnimationFrame(() => {
            pitexScrollTick = false;
            if (Date.now() < pitexSuppressUntil) return;
            const bridge = window.webkit && window.webkit.messageHandlers &&
                window.webkit.messageHandlers.pitexScroll;
            if (bridge) bridge.postMessage(pitexTopLine());
        });
    }, { passive: true });
}
