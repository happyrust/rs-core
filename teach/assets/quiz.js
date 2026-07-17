/**
 * Minimal quiz: mark picked, show feedback. Options must be same length by authoring.
 */
(function () {
  function bind(root) {
    root.querySelectorAll(".q").forEach(function (block) {
      var feedback = block.querySelector(".feedback");
      var buttons = block.querySelectorAll("button[data-correct]");
      buttons.forEach(function (btn) {
        btn.addEventListener("click", function () {
          if (block.dataset.done === "1") return;
          block.dataset.done = "1";
          buttons.forEach(function (b) { b.classList.remove("picked"); });
          btn.classList.add("picked");
          var ok = btn.getAttribute("data-correct") === "1";
          feedback.textContent = ok
            ? (btn.getAttribute("data-ok") || "正确。")
            : (btn.getAttribute("data-bad") || "再想一想，对照决策树。");
          feedback.className = "feedback " + (ok ? "ok" : "bad");
        });
      });
    });
  }
  document.addEventListener("DOMContentLoaded", function () {
    document.querySelectorAll(".quiz").forEach(bind);
  });
})();
