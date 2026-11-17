// Yanshi Landing Page Interactive Controller
// Zero dependencies. Handles bilingual switching, color-scheme override,
// accessible tablists (Studio pipeline + 4 execution paradigms), catalogue filtering, and clipboard copy.

(() => {
  const TITLES = {
    zh: "Yanshi 偃师 · 确定性工作流与智能体多模态内容生产引擎",
    en: "Yanshi · Multimodal Content-Production Runtime in Rust",
  };

  const html = document.documentElement;
  const schemeMeta = document.querySelector('meta[name="color-scheme"]');
  const darkMql = window.matchMedia("(prefers-color-scheme: dark)");

  function getLang() {
    return html.dataset.lang === "zh" ? "zh" : "en";
  }

  function applyLang(lang, persist) {
    const normalized = lang === "zh" ? "zh" : "en";
    html.dataset.lang = normalized;
    html.lang = normalized === "zh" ? "zh-CN" : "en";
    document.title = TITLES[normalized];
    if (persist) {
      try {
        localStorage.setItem("yanshi-lang", normalized);
      } catch {
        // Ignore storage errors in restricted browsing modes
      }
    }
  }

  // Sync initial document title with head-resolved language
  document.title = TITLES[getLang()];

  const langBtn = document.getElementById("lang-toggle");
  if (langBtn) {
    langBtn.addEventListener("click", () => {
      applyLang(getLang() === "zh" ? "en" : "zh", true);
    });
  }

  // Color-scheme toggle following modern-web-guidance two-state model:
  // pinned override ("light" | "dark") or system default ("light dark", no storage key)
  function getStoredScheme() {
    try {
      const val = localStorage.getItem("yanshi-color-scheme");
      return val === "light" || val === "dark" ? val : null;
    } catch {
      return null;
    }
  }

  function getRenderedScheme() {
    const metaVal = schemeMeta ? schemeMeta.content : "light dark";
    if (metaVal === "light" || metaVal === "dark") {
      return metaVal;
    }
    return darkMql.matches ? "dark" : "light";
  }

  const themeBtn = document.getElementById("theme-toggle");
  if (themeBtn && schemeMeta) {
    themeBtn.addEventListener("click", () => {
      const current = getRenderedScheme();
      const target = current === "dark" ? "light" : "dark";
      const system = darkMql.matches ? "dark" : "light";

      if (target === system) {
        try {
          localStorage.removeItem("yanshi-color-scheme");
        } catch {
          // Ignore storage restrictions
        }
        schemeMeta.content = "light dark";
      } else {
        try {
          localStorage.setItem("yanshi-color-scheme", target);
        } catch {
          // Ignore storage restrictions
        }
        schemeMeta.content = target;
      }
    });
  }

  window.addEventListener("storage", (event) => {
    if (event.key === "yanshi-color-scheme" && schemeMeta) {
      const next = event.newValue === "light" || event.newValue === "dark"
        ? event.newValue
        : "light dark";
      schemeMeta.content = next;
    } else if (event.key === "yanshi-lang" && event.newValue) {
      applyLang(event.newValue, false);
    }
  });

  // Accessible Tablists (Selection follows focus + hidden="until-found" find-in-page sync)
  function initTabGroup(container) {
    const tablist = container.querySelector('[role="tablist"]');
    if (!tablist) {
      return;
    }
    const tabs = Array.from(tablist.querySelectorAll('[role="tab"]'));
    const panels = tabs
      .map((tab) => document.getElementById(tab.dataset.panel || ""))
      .filter(Boolean);

    function keepTabVisible(tab) {
      if (tablist.scrollWidth <= tablist.clientWidth) {
        return;
      }
      const listRect = tablist.getBoundingClientRect();
      const tabRect = tab.getBoundingClientRect();
      if (tabRect.left < listRect.left) {
        tablist.scrollLeft -= listRect.left - tabRect.left + 16;
      } else if (tabRect.right > listRect.right) {
        tablist.scrollLeft += tabRect.right - listRect.right + 16;
      }
    }

    function selectTab(selected, moveFocus) {
      tabs.forEach((tab, idx) => {
        const isSelected = tab === selected;
        tab.setAttribute("aria-selected", isSelected ? "true" : "false");
        tab.tabIndex = isSelected ? 0 : -1;
        const panel = panels[idx];
        if (panel) {
          if (isSelected) {
            panel.removeAttribute("hidden");
          } else {
            panel.setAttribute("hidden", "until-found");
          }
        }
      });
      keepTabVisible(selected);
      if (moveFocus) {
        selected.focus();
      }
    }

    tabs.forEach((tab, idx) => {
      tab.addEventListener("click", () => selectTab(tab, false));
      tab.addEventListener("keydown", (event) => {
        let nextIdx = -1;
        if (event.key === "ArrowRight" || event.key === "ArrowDown") {
          nextIdx = (idx + 1) % tabs.length;
        } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
          nextIdx = (idx - 1 + tabs.length) % tabs.length;
        } else if (event.key === "Home") {
          nextIdx = 0;
        } else if (event.key === "End") {
          nextIdx = tabs.length - 1;
        }
        if (nextIdx >= 0) {
          event.preventDefault();
          selectTab(tabs[nextIdx], true);
        }
      });

      const panel = panels[idx];
      if (panel) {
        panel.addEventListener("beforematch", () => selectTab(tab, false));
      }
    });
  }

  document.querySelectorAll("[data-tab-group]").forEach(initTabGroup);

  // Studio 15-Template Catalogue Filter
  const filterBtns = Array.from(document.querySelectorAll("[data-filter]"));
  const tplItems = Array.from(document.querySelectorAll(".tpl-item[data-cat]"));

  filterBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      const cat = btn.dataset.filter || "all";
      filterBtns.forEach((other) => {
        other.setAttribute("aria-pressed", other === btn ? "true" : "false");
      });
      tplItems.forEach((item) => {
        const match = cat === "all" || item.dataset.cat === cat;
        item.hidden = !match;
      });
    });
  });

  // Clipboard Copy Buttons
  document.querySelectorAll("[data-copy-source]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const targetId = btn.dataset.copySource;
      const sourceEl = targetId ? document.getElementById(targetId) : null;
      const text = sourceEl ? sourceEl.textContent.trim() : "";
      if (!text || !navigator.clipboard) {
        return;
      }
      try {
        await navigator.clipboard.writeText(text);
        btn.dataset.copied = "true";
        const zhSpan = btn.querySelector("[data-i18n-zh]");
        const enSpan = btn.querySelector("[data-i18n-en]");
        const prevZh = zhSpan ? zhSpan.textContent : "";
        const prevEn = enSpan ? enSpan.textContent : "";
        if (zhSpan) {
          zhSpan.textContent = "已复制";
        }
        if (enSpan) {
          enSpan.textContent = "Copied";
        }
        window.setTimeout(() => {
          delete btn.dataset.copied;
          if (zhSpan) {
            zhSpan.textContent = prevZh;
          }
          if (enSpan) {
            enSpan.textContent = prevEn;
          }
        }, 1600);
      } catch {
        // Ignore clipboard permission denial
      }
    });
  });
})();
