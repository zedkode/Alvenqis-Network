/* FOUC theme bootstrap — external so CSP can use script-src 'self' only. */
(function () {
  try {
    var key = "alvenqis.theme";
    var t = localStorage.getItem(key);
    if (t !== "dark" && t !== "light") {
      t =
        window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches
          ? "light"
          : "dark";
    }
    document.documentElement.setAttribute("data-theme", t);
    document.documentElement.style.colorScheme = t;
    var meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute("content", t === "light" ? "#eef4f8" : "#030c12");
  } catch (e) {
    document.documentElement.setAttribute("data-theme", "dark");
  }
})();
