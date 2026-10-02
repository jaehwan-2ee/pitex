'use strict';
// Equation preview renderer page — loaded once per renderer lifecycle into
// a persistent hidden/popover web view. Native code calls only
//   await window.pitexEquation.ready            -> identity string
//   await window.pitexEquation.render(request)  -> { ok, svg } | { ok:false, error }
//   window.pitexEquation.show(presentation)     -> { width, height }
//   window.pitexEquation.clear()
// with structured arguments (callAsyncJavaScript / call_async_javascript_function);
// TeX never becomes script text.
(() => {
  const MAX_SOURCE = 32 * 1024;
  const MAX_DEFINITIONS = 4096;
  const MAX_SVG = 4 * 1024 * 1024;

  const display = document.getElementById('display');
  const measure = document.getElementById('measure');
  let queue = Promise.resolve();

  // ---- SVG allowlist sanitizer (independent of ui/safe) -------------------
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const XLINK_NS = 'http://www.w3.org/1999/xlink';
  const ELEMENTS = new Set([
    'svg', 'g', 'path', 'use', 'defs', 'rect', 'line', 'polyline', 'polygon',
    'circle', 'ellipse', 'text', 'tspan', 'title', 'desc', 'symbol',
  ]);
  // Unwrapped (children kept): links from \href.
  const UNWRAP = new Set(['a']);
  const ATTRIBUTES = new Set([
    'd', 'fill', 'stroke', 'stroke-width', 'stroke-linecap', 'stroke-linejoin',
    'stroke-dasharray', 'stroke-miterlimit', 'fill-opacity', 'stroke-opacity', 'opacity',
    'transform', 'x', 'y', 'x1', 'y1', 'x2', 'y2', 'dx', 'dy', 'cx', 'cy', 'r', 'rx', 'ry',
    'width', 'height', 'viewBox', 'points', 'preserveAspectRatio', 'overflow', 'visibility',
    'font-family', 'font-size', 'font-weight', 'font-style', 'text-anchor',
    'dominant-baseline', 'alignment-baseline', 'role', 'focusable', 'aria-hidden', 'xmlns',
  ]);
  const STYLE_PROPERTIES = new Set([
    'vertical-align', 'margin-left', 'margin-right', 'margin-top', 'margin-bottom',
    'min-width', 'max-width', 'width', 'height', 'overflow', 'display',
    'color', 'fill', 'stroke', 'background-color', 'opacity',
    'font-family', 'font-size', 'font-weight', 'font-style',
  ]);
  const UNSAFE_VALUE = /url\s*\(|expression\s*\(|javascript:|@import|\\|<|>/i;

  function sanitizeStyle(value) {
    const kept = [];
    for (const declaration of value.split(';')) {
      const colon = declaration.indexOf(':');
      if (colon < 0) continue;
      const property = declaration.slice(0, colon).trim().toLowerCase();
      const propertyValue = declaration.slice(colon + 1).trim();
      if (STYLE_PROPERTIES.has(property) && !UNSAFE_VALUE.test(propertyValue)) {
        kept.push(property + ': ' + propertyValue);
      }
    }
    return kept.join('; ');
  }

  function sanitizeElement(element) {
    for (const child of Array.from(element.children)) {
      const name = child.localName;
      if (child.namespaceURI !== SVG_NS) { child.remove(); continue; }
      if (UNWRAP.has(name)) {
        sanitizeElement(child);
        child.replaceWith(...Array.from(child.childNodes));
        continue;
      }
      if (!ELEMENTS.has(name)) { child.remove(); continue; }
      sanitizeElement(child);
    }
    for (const attribute of Array.from(element.attributes)) {
      const name = attribute.name;
      const value = attribute.value;
      let keep = false;
      if (name === 'href' || name === 'xlink:href') {
        // fontCache:'local' glyph references only.
        keep = element.localName === 'use' && /^#MJX-[\w.-]+$/.test(value);
      } else if (name === 'id') {
        keep = /^MJX-[\w.-]+$/.test(value);
      } else if (name === 'style') {
        const style = sanitizeStyle(value);
        if (style) element.setAttribute('style', style);
        continue;
      } else if (name === 'class') {
        const classes = value.split(/\s+/).filter((c) => /^mjx-[\w.-]+$/.test(c));
        if (classes.length) element.setAttribute('class', classes.join(' '));
        else element.removeAttribute('class');
        continue;
      } else if (name.startsWith('data-')) {
        keep = !UNSAFE_VALUE.test(value);
      } else if (name === 'xmlns:xlink') {
        keep = value === XLINK_NS;
      } else {
        keep = ATTRIBUTES.has(name) && !UNSAFE_VALUE.test(value);
      }
      if (!keep) element.removeAttributeNS(attribute.namespaceURI, attribute.localName);
    }
  }

  /** Sanitized serialization of a MathJax SVG root. */
  function sanitizedSVG(svg) {
    const copy = svg.cloneNode(true);
    const wrapper = document.createElementNS(SVG_NS, 'g');
    wrapper.appendChild(copy);
    sanitizeElement(wrapper);
    const root = wrapper.firstElementChild;
    if (!root || root.localName !== 'svg') return '';
    root.setAttribute('aria-hidden', 'true');
    root.setAttribute('focusable', 'false');
    return new XMLSerializer().serializeToString(root);
  }

  /** Re-parse and re-sanitize markup handed back by native code. */
  function parseSVG(markup) {
    if (typeof markup !== 'string' || markup.length === 0 || markup.length > MAX_SVG) return null;
    const parsed = new DOMParser().parseFromString(markup, 'image/svg+xml');
    const root = parsed.documentElement;
    if (!root || root.localName !== 'svg' || root.namespaceURI !== SVG_NS) return null;
    const clean = sanitizedSVG(root);
    if (!clean) return null;
    return document.importNode(new DOMParser().parseFromString(clean, 'image/svg+xml').documentElement, true);
  }

  // ---- MathJax ------------------------------------------------------------
  function classify(error) {
    const id = error && typeof error.id === 'string' ? error.id : null;
    if (id === 'UndefinedControlSequence' || id === 'UnknownEnv') return 'undefined';
    if (id && /^Max/.test(id)) return 'limit';
    return id ? 'tex' : 'internal';
  }

  async function renderNow(request) {
    const source = request && request.source;
    const definitions = Array.isArray(request && request.definitions) ? request.definitions : [];
    const displayMode = Boolean(request && request.displayMode);
    const fontSize = Number(request && request.fontSize) || 16;
    if (typeof source !== 'string') return { ok: false, error: 'internal' };
    if (source.length > MAX_SOURCE) return { ok: false, error: 'limit' };
    // A fresh protected sandbox per render: nothing a previous equation or
    // a hostile definition did (\endgroup, \gdef, leftover groups) can leak
    // into this one, whatever context it claims. The runtime and fonts stay
    // loaded; only the TeX group state resets.
    MathJax.tex2mml('\\begingroupSandbox');
    // Definitions replay inside the sandbox. One joined parse is fast even
    // with hundreds of definitions; if any of it throws, a fresh sandbox is
    // rebuilt before the per-definition pass so a half-applied batch can
    // never contaminate the retry.
    const defs = definitions.filter((d) => typeof d === 'string').slice(0, MAX_DEFINITIONS);
    if (defs.length) {
      try {
        await MathJax.tex2mmlPromise(defs.join('\n'));
      } catch (_) {
        MathJax.tex2mml('\\begingroupSandbox');
        for (const definition of defs) {
          try {
            await MathJax.tex2mmlPromise(definition);
          } catch (_) {
            // One malformed definition must not drop the rest.
          }
        }
      }
    }
    // Per-render group: \newcommand inside the equation stays local to it.
    MathJax.tex2mml('\\begingroupReset\\begingroup');
    MathJax.texReset();
    measure.style.fontSize = fontSize + 'px';
    try {
      const node = await MathJax.tex2svgPromise(source, {
        ...MathJax.getMetricsFor(measure, displayMode),
        // The hidden metric box is narrow; never let it drive layout.
        containerWidth: 1e6,
        display: displayMode,
      });
      const svg = node.querySelector('svg');
      const markup = svg ? sanitizedSVG(svg) : '';
      if (!markup) return { ok: false, error: 'internal', detail: svg ? 'sanitizer' : 'no svg' };
      if (markup.length > MAX_SVG) return { ok: false, error: 'limit' };
      return { ok: true, svg: markup };
    } catch (error) {
      // `detail` is for developer logs only; the UI never shows it.
      return { ok: false, error: classify(error), detail: String((error && (error.id || error.message)) || error) };
    }
  }

  const ready = MathJax.startup.promise.then(() => {
    // Sandbox before anything user-provided is ever parsed.
    MathJax.tex2mml('\\begingroupSandbox');
    const identity = window.PITEX_EQUATION_IDENTITY;
    return 'mathjax-' + MathJax.version + '+' + identity.font + '+page-' + identity.pageRevision;
  });

  window.pitexEquation = Object.freeze({
    ready,
    render(request) {
      // Serialized: interleaved renders would share sandbox/group state.
      const result = queue.then(() => ready).then(() => renderNow(request));
      queue = result.catch(() => undefined);
      return result.catch((error) => ({
        ok: false, error: 'internal', detail: String((error && (error.id || error.message)) || error),
      }));
    },
    show(presentation) {
      const theme = (presentation && presentation.theme) || {};
      const root = document.documentElement.style;
      if (typeof theme.foreground === 'string') root.setProperty('--pitex-fg', theme.foreground);
      if (typeof theme.background === 'string') root.setProperty('--pitex-bg', theme.background);
      const fontSize = Number(presentation && presentation.fontSize) || 16;
      root.setProperty('--pitex-font-size', fontSize + 'px');
      const svg = parseSVG(presentation && presentation.svg);
      display.replaceChildren(...(svg ? [svg] : []));
      window.scrollTo(0, 0);
      const rect = display.getBoundingClientRect();
      return { width: Math.ceil(rect.width), height: Math.ceil(rect.height), ok: Boolean(svg) };
    },
    clear() {
      display.replaceChildren();
    },
  });
})();
