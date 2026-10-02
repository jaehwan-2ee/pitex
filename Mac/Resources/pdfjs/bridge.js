// bridge.js — classic script loaded BEFORE viewer.mjs. Installs the
// console / CSP / error hooks so a module link or parse failure in
// viewer.mjs is still reported to the host instead of timing out silently.
(function () {
  var post = function (m) {
    try { window.webkit && window.webkit.messageHandlers &&
            window.webkit.messageHandlers.pitexPdf &&
            window.webkit.messageHandlers.pitexPdf.postMessage(m); } catch (_) {}
  };
  window.__pitexPost = post;
  ["warn", "error", "log"].forEach(function (level) {
    var orig = console[level].bind(console);
    console[level] = function () {
      var a = Array.prototype.slice.call(arguments);
      try { post({type: "console", level: level, text: a.join(" ")}); } catch (_) {}
      return orig.apply(null, a);
    };
  });
  document.addEventListener("securitypolicyviolation", function (e) {
    post({type: "csp", blocked: e.blockedURI, directive: e.violatedDirective});
  });
  window.addEventListener("error", function (e) {
    post({type: "jserror", text: String(e.message || e)});
  });
  window.addEventListener("unhandledrejection", function (e) {
    post({type: "jserror", text: "unhandledrejection: " + String(e.reason)});
  });
})();
