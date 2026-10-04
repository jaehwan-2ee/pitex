'use strict';
// MathJax configuration — must run before vendor/mathjax/tex-svg-nofont.js.
// Every loader path is local to the pitex-equation:// asset tree: the
// upstream font default is a CDN, and the default font (NewCM) is not
// vendored, so both `paths.fonts` and `output.font` are mandatory.
(() => {
  const vendor = new URL('vendor/mathjax', document.baseURI).href;

  // Renderer identity — part of every cache key. Bump PAGE_REVISION when
  // renderer.js output changes; FONT tracks Assets/equation-preview/vendor/VERSIONS.
  window.PITEX_EQUATION_IDENTITY = {
    pageRevision: 2,
    font: 'mathjax-tex@4.1.3',
  };

  window.MathJax = {
    loader: {
      paths: { mathjax: vendor, fonts: vendor },
      // safe: URL/style/class/id filtering for \href, \style, \class …
      // begingroup: \begingroupSandbox isolates user definitions per context.
      load: ['ui/safe', '[tex]/begingroup'],
    },
    tex: {
      // noundefined would render unknown macros in red instead of failing,
      // hiding "unsupported" from the preview engine.
      packages: { '[+]': ['begingroup'], '[-]': ['noundefined'] },
      // Throw instead of typesetting an merror node.
      formatError: (jax, error) => { throw error; },
      maxMacros: 2000,
      maxBuffer: 64 * 1024,
      protectedMacros: ['begingroupSandbox', 'begingroupReset', 'begingroup', 'endgroup'],
    },
    output: { font: 'mathjax-tex' },
    // TeX never breaks display math by itself; large expressions scroll.
    svg: { fontCache: 'local', displayAlign: 'left', displayIndent: '0', linebreaks: { inline: false } },
    options: {
      // a11y/SRE are not vendored; native accessibility carries the label
      // and the TeX source instead.
      enableMenu: false,
      enableEnrichment: false,
      enableComplexity: false,
      enableSpeech: false,
      enableBraille: false,
      enableExplorer: false,
      enableExplorerHelp: false,
      enableAssistiveMml: false,
      // ui/menu re-derives the flags above from its own settings (speech,
      // braille, enrichment default on) — they would wait forever for the
      // unvendored SRE, so switch them off at the source as well.
      menuOptions: {
        settings: {
          enrich: false, speech: false, braille: false, collapsible: false,
          assistiveMml: false, inTabOrder: false, help: false,
        },
      },
      safeOptions: {
        allow: { URLs: 'none', classes: 'safe', cssIDs: 'safe', styles: 'safe' },
      },
    },
    startup: { typeset: false },
  };
})();
