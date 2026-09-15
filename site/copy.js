(function () {
  "use strict";
  document.querySelectorAll(".copy").forEach(function (btn) {
    btn.setAttribute("aria-live", "polite");
    btn.addEventListener("click", function () {
      var text = btn.getAttribute("data-copy");
      var done = function (label) {
        btn.textContent = label;
        setTimeout(function () { btn.textContent = "Copy"; }, 1400);
      };
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(function () { done("Copied"); }, function () { done("Press ⌘C"); });
      } else {
        done("Press ⌘C");
      }
    });
  });
})();
