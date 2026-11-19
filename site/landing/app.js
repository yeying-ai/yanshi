// Yanshi Landing Page Interactive Controller — Enhanced Edition
// Zero dependencies. Handles bilingual switching, color-scheme override,
// syntax highlighting (YAML / Rust / Bash), spotlight hover cards, accessible tablists,
// catalogue filtering, and in-page <dialog> YAML workflow inspector.

(() => {
  const TITLES = {
    zh: "Yanshi 偃师 · Rust 确定性 DAG 工作流与智能体多模态内容生产引擎",
    en: "Yanshi · Deterministic DAG Workflow & Multimodal AI Agent Runtime in Rust",
  };

  const html = document.documentElement;
  const assetPrefix = html.dataset.assetPrefix || "";
  const schemeMeta = document.querySelector('meta[name="color-scheme"]');
  const darkMql = window.matchMedia("(prefers-color-scheme: dark)");

  function escapeHtml(str) {
    return str
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  // Single-pass tokenizing syntax highlighter (XSS-safe, zero regex overlap)
  function highlightYamlValue(text) {
    const tokenRe =
      /(\{\{[\s\S]*?\}\}|\{%[\s\S]*?%\}|\$\{[^}]+\}|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\b(?:true|false|null)\b|\b\d+(?:\.\d+)?\b|#.*$)/g;
    let out = "";
    let last = 0;
    let match;
    while ((match = tokenRe.exec(text)) !== null) {
      out += escapeHtml(text.slice(last, match.index));
      const tok = match[0];
      if (tok.startsWith("#")) {
        out += `<span class="tok-cmt">${escapeHtml(tok)}</span>`;
      } else if (tok.startsWith("{{") || tok.startsWith("{%") || tok.startsWith("${")) {
        out += `<span class="tok-expr">${escapeHtml(tok)}</span>`;
      } else if (tok.startsWith('"') || tok.startsWith("'")) {
        // Highlight embedded {{...}} or ${...} inside quoted YAML strings
        const inner = tok.replace(
          /(\{\{[\s\S]*?\}\}|\{%[\s\S]*?%\}|\$\{[^}]+\})/g,
          "\0$1\0"
        );
        const parts = inner.split("\0");
        const rendered = parts
          .map((part, idx) =>
            idx % 2 === 1
              ? `<span class="tok-expr">${escapeHtml(part)}</span>`
              : escapeHtml(part)
          )
          .join("");
        out += `<span class="tok-str">${rendered}</span>`;
      } else if (tok === "true" || tok === "false" || tok === "null") {
        out += `<span class="tok-kw">${tok}</span>`;
      } else {
        out += `<span class="tok-num">${tok}</span>`;
      }
      last = tokenRe.lastIndex;
    }
    out += escapeHtml(text.slice(last));
    return out;
  }

  function highlightYamlLine(line) {
    if (/^\s*#/.test(line)) {
      return `<span class="tok-cmt">${escapeHtml(line)}</span>`;
    }
    const keyMatch = line.match(/^(\s*(?:-\s+)?)([a-zA-Z0-9_.-]+)(:)(\s*.*)$/);
    if (keyMatch) {
      const [, prefix, key, colon, rest] = keyMatch;
      const prefixHtml = prefix.replace(
        /-/,
        '<span class="tok-punc">-</span>'
      );
      return (
        `${prefixHtml}<span class="tok-key">${escapeHtml(key)}</span>` +
        `<span class="tok-punc">${colon}</span>${highlightYamlValue(rest)}`
      );
    }
    const listMatch = line.match(/^(\s*-\s+)(.+)$/);
    if (listMatch) {
      return `<span class="tok-punc">${escapeHtml(listMatch[1])}</span>${highlightYamlValue(listMatch[2])}`;
    }
    return highlightYamlValue(line);
  }

  function highlightRust(code) {
    const rustRe =
      /(\/\/[^\n]*|"(?:\\.|[^"\\])*"|\b(?:use|let|mut|pub|fn|async|await|return|impl|struct|enum|const|for|in|if|else|match)\b|\b[A-Z][A-Za-z0-9_]*\b|\b\d+(?:\.\d+)?\b)/g;
    let out = "";
    let last = 0;
    let match;
    while ((match = rustRe.exec(code)) !== null) {
      out += escapeHtml(code.slice(last, match.index));
      const tok = match[0];
      if (tok.startsWith("//")) {
        out += `<span class="tok-cmt">${escapeHtml(tok)}</span>`;
      } else if (tok.startsWith('"')) {
        out += `<span class="tok-str">${escapeHtml(tok)}</span>`;
      } else if (/^[A-Z]/.test(tok)) {
        out += `<span class="tok-type">${escapeHtml(tok)}</span>`;
      } else if (/^\d/.test(tok)) {
        out += `<span class="tok-num">${tok}</span>`;
      } else {
        out += `<span class="tok-kw">${tok}</span>`;
      }
      last = rustRe.lastIndex;
    }
    out += escapeHtml(code.slice(last));
    return out;
  }

  function highlightBash(code) {
    const bashRe =
      /(#[^\n]*|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|--[a-zA-Z0-9_-]+|\b(?:yanshi|cargo)\b|\b(?:workflow|notebook|skill|run|resume|validate|install|trace|replay)\b|(?:https?:\/\/|examples\/|\.\/scripts\/)[^\s\\]+)/g;
    let out = "";
    let last = 0;
    let match;
    while ((match = bashRe.exec(code)) !== null) {
      out += escapeHtml(code.slice(last, match.index));
      const tok = match[0];
      if (tok.startsWith("#")) {
        out += `<span class="tok-cmt">${escapeHtml(tok)}</span>`;
      } else if (tok.startsWith("--")) {
        out += `<span class="tok-expr">${escapeHtml(tok)}</span>`;
      } else if (tok === "yanshi" || tok === "cargo") {
        out += `<span class="tok-type">${tok}</span>`;
      } else if (
        tok.startsWith('"') ||
        tok.startsWith("'") ||
        tok.startsWith("http") ||
        tok.startsWith("examples/") ||
        tok.startsWith("./scripts/")
      ) {
        out += `<span class="tok-str">${escapeHtml(tok)}</span>`;
      } else {
        out += `<span class="tok-kw">${escapeHtml(tok)}</span>`;
      }
      last = bashRe.lastIndex;
    }
    out += escapeHtml(code.slice(last));
    return out;
  }

  function highlightBlock(code, lang) {
    if (lang === "rust") {
      return highlightRust(code);
    }
    if (lang === "bash") {
      return highlightBash(code);
    }
    return code.split("\n").map(highlightYamlLine).join("\n");
  }

  // Apply syntax highlighting to all static code blocks while preserving raw copy text
  document.querySelectorAll("code[data-hl]").forEach((codeEl) => {
    const promptEl = codeEl.querySelector(".cli-prompt");
    const prefixHtml = promptEl ? promptEl.outerHTML : "";
    const raw = promptEl
      ? codeEl.textContent.replace(/^\$\s*/, "").trim()
      : codeEl.textContent;
    codeEl.dataset.rawCode = raw;
    codeEl.innerHTML = prefixHtml + highlightBlock(raw, codeEl.dataset.hl);
  });

  // Bilingual route & fallback controller
  function getLang() {
    return html.dataset.lang === "zh" ? "zh" : "en";
  }

  let activeWorkflowKey = null;

  function applyLang(lang) {
    const normalized = lang === "zh" ? "zh" : "en";
    html.dataset.lang = normalized;
    html.lang = normalized === "zh" ? "zh-CN" : "en";
    document.title = TITLES[normalized];
    if (activeWorkflowKey) {
      updateModalStatsText(activeWorkflowKey);
    }
  }

  const langBtn = document.getElementById("lang-toggle");
  const isPreRenderedSingleLang =
    !document.querySelector("[data-i18n-zh]") ||
    !document.querySelector("[data-i18n-en]");

  if (langBtn) {
    const baseHref = langBtn.getAttribute("href") || "en/";
    const syncLangHref = () => {
      if (!isPreRenderedSingleLang) {
        return;
      }
      let targetBase = baseHref;
      if (location.protocol === "file:" && targetBase.endsWith("/")) {
        targetBase += "index.html";
      }
      try {
        const params = new URLSearchParams(location.search);
        params.delete("lang");
        const qs = params.toString();
        langBtn.setAttribute(
          "href",
          targetBase + (qs ? `?${qs}` : "") + location.hash
        );
      } catch {}
    };
    syncLangHref();
    langBtn.addEventListener("pointerenter", syncLangHref, { passive: true });
    langBtn.addEventListener("focus", syncLangHref);
    langBtn.addEventListener("click", (event) => {
      if (!isPreRenderedSingleLang) {
        event.preventDefault();
        applyLang(getLang() === "zh" ? "en" : "zh");
        return;
      }
      syncLangHref();
    });
  }

  // Color-scheme toggle (modern-web-guidance two-state model)
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
        } catch {}
        schemeMeta.content = "light dark";
      } else {
        try {
          localStorage.setItem("yanshi-color-scheme", target);
        } catch {}
        schemeMeta.content = target;
      }
    });
  }

  window.addEventListener("storage", (event) => {
    if (event.key === "yanshi-color-scheme" && schemeMeta) {
      const next =
        event.newValue === "light" || event.newValue === "dark"
          ? event.newValue
          : "light dark";
      schemeMeta.content = next;
    }
  });

  // Cursor-tracked radial spotlight on cards
  document.querySelectorAll(".spotlight-card").forEach((card) => {
    card.addEventListener(
      "pointermove",
      (event) => {
        const rect = card.getBoundingClientRect();
        card.style.setProperty("--mx", `${event.clientX - rect.left}px`);
        card.style.setProperty("--my", `${event.clientY - rect.top}px`);
      },
      { passive: true }
    );
  });

  // Accessible Tablists
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

  // Studio 19-Template Catalogue Filter
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

  // Reusable Copy Feedback Helper
  async function copyWithFeedback(btn, text) {
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
      }, 1500);
    } catch {
      // Ignore clipboard denial
    }
  }

  document.querySelectorAll("[data-copy-source]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const targetId = btn.dataset.copySource;
      const sourceEl = targetId ? document.getElementById(targetId) : null;
      if (!sourceEl) {
        return;
      }
      const raw = sourceEl.dataset.rawCode || sourceEl.textContent.trim();
      copyWithFeedback(btn, raw);
    });
  });

  // In-Page <dialog> Workflow YAML Inspector
  const modal = document.getElementById("workflow-modal");
  const modalTitle = document.getElementById("wf-modal-title");
  const modalStats = document.getElementById("wf-modal-stats");
  const modalDagNodes = document.getElementById("wf-dag-nodes");
  const modalCliCode = document.getElementById("wf-modal-cli");
  const modalCodeBody = document.getElementById("wf-code-tbody");
  const modalCodeScroll = document.getElementById("wf-code-scroll");
  const modalDownload = document.getElementById("wf-download-link");
  const modalGithub = document.getElementById("wf-github-link");
  const modalCopyYaml = document.getElementById("wf-copy-yaml");
  const modalCopyCli = document.getElementById("wf-copy-cli");
  const modalPrev = document.getElementById("wf-prev-btn");
  const modalNext = document.getElementById("wf-next-btn");

  const workflowOrder = tplItems
    .map((el) => el.dataset.workflow)
    .filter(Boolean);

  // Lazy-load 203 KB workflows-data.js off the critical rendering path
  let workflowsPromise = null;

  function ensureWorkflowsLoaded() {
    if (window.YANSHI_WORKFLOWS) {
      return Promise.resolve(window.YANSHI_WORKFLOWS);
    }
    if (!workflowsPromise) {
      workflowsPromise = new Promise((resolve, reject) => {
        const script = document.createElement("script");
        script.src = `${assetPrefix}workflows-data.js`;
        script.async = true;
        script.setAttribute("fetchpriority", "low");
        script.onload = () => resolve(window.YANSHI_WORKFLOWS || {});
        script.onerror = () => {
          workflowsPromise = null;
          reject(new Error("Failed to load workflows-data.js"));
        };
        document.head.appendChild(script);
      });
    }
    return workflowsPromise;
  }

  const catalogueSection = document.getElementById("catalogue");
  if (catalogueSection) {
    catalogueSection.addEventListener(
      "pointerenter",
      () => {
        ensureWorkflowsLoaded().catch(() => {});
      },
      { once: true, passive: true }
    );
    catalogueSection.addEventListener(
      "focusin",
      () => {
        ensureWorkflowsLoaded().catch(() => {});
      },
      { once: true }
    );
    if ("IntersectionObserver" in window) {
      const observer = new IntersectionObserver(
        (entries) => {
          if (entries.some((entry) => entry.isIntersecting)) {
            observer.disconnect();
            ensureWorkflowsLoaded().catch(() => {});
          }
        },
        { rootMargin: "320px 0px" }
      );
      observer.observe(catalogueSection);
    }
  }

  document.querySelectorAll('a[href="#catalogue"]').forEach((anchor) => {
    anchor.addEventListener(
      "pointerenter",
      () => {
        ensureWorkflowsLoaded().catch(() => {});
      },
      { once: true, passive: true }
    );
  });

  function syncWorkflowQuery(filename) {
    if (location.protocol !== "http:" && location.protocol !== "https:") {
      return;
    }
    try {
      const url = new URL(location.href);
      if (filename) {
        url.searchParams.set("workflow", filename);
      } else {
        url.searchParams.delete("workflow");
      }
      history.replaceState(null, "", url.pathname + url.search + url.hash);
    } catch {}
  }

  function getWorkflowData(filename) {
    const store = window.YANSHI_WORKFLOWS || {};
    return store[filename] || null;
  }

  function updateModalStatsText(filename) {
    const wf = getWorkflowData(filename);
    if (!wf || !modalStats) {
      return;
    }
    const nNodes = Array.isArray(wf.nodes) ? wf.nodes.length : 0;
    modalStats.textContent =
      getLang() === "zh"
        ? `${wf.lines} 行 · ${nNodes} 个 DAG 节点`
        : `${wf.lines} lines · ${nNodes} DAG nodes`;
  }

  function renderWorkflowIntoModal(filename) {
    const wf = getWorkflowData(filename);
    if (!wf || !modal) {
      return false;
    }
    activeWorkflowKey = filename;

    if (modalTitle) {
      modalTitle.textContent = wf.path;
    }
    updateModalStatsText(filename);

    // Render visual DAG node pipeline strip
    if (modalDagNodes) {
      const nodes = Array.isArray(wf.nodes) ? wf.nodes : [];
      modalDagNodes.innerHTML = nodes
        .map((n, i) => {
          const arrow =
            i < nodes.length - 1
              ? '<span class="wf-node-arrow" aria-hidden="true">→</span>'
              : "";
          return (
            `<span class="wf-node-pill">` +
            `<span class="wf-node-id">${escapeHtml(n.id)}</span>` +
            `<span class="wf-node-type">${escapeHtml(n.type)}</span>` +
            `</span>${arrow}`
          );
        })
        .join("");
    }

    // Render CLI run bar
    if (modalCliCode) {
      modalCliCode.dataset.rawCode = wf.cli;
      modalCliCode.innerHTML =
        '<span class="cli-prompt">$</span>' + highlightBash(wf.cli);
    }

    // Render Download & GitHub links
    if (modalDownload) {
      modalDownload.href = `${assetPrefix}workflows/${encodeURIComponent(wf.file)}`;
      modalDownload.setAttribute("download", wf.file);
    }
    if (modalGithub) {
      modalGithub.href = `https://github.com/yeying-ai/yanshi/blob/main/${wf.path}`;
    }

    // Render line-numbered syntax-highlighted YAML
    if (modalCodeBody) {
      const yamlLines = wf.yaml.replace(/\n$/, "").split("\n");
      modalCodeBody.innerHTML = yamlLines
        .map(
          (line, idx) =>
            `<tr><td class="wf-ln">${idx + 1}</td>` +
            `<td class="wf-lc">${highlightYamlLine(line)}</td></tr>`
        )
        .join("");
    }
    if (modalCodeScroll) {
      modalCodeScroll.scrollTop = 0;
      modalCodeScroll.scrollLeft = 0;
    }
    syncWorkflowQuery(filename);
    return true;
  }

  async function openWorkflowModal(filename) {
    if (!modal) {
      return;
    }
    try {
      await ensureWorkflowsLoaded();
    } catch {
      // Graceful fallback: navigate directly to the crawlable .yml file
      location.href = `${assetPrefix}workflows/${encodeURIComponent(filename)}`;
      return;
    }
    if (!renderWorkflowIntoModal(filename)) {
      return;
    }
    if (!modal.open) {
      modal.showModal();
    }
  }

  function stepWorkflow(delta) {
    if (!activeWorkflowKey || workflowOrder.length === 0) {
      return;
    }
    const idx = workflowOrder.indexOf(activeWorkflowKey);
    const nextIdx =
      (idx + delta + workflowOrder.length) % workflowOrder.length;
    renderWorkflowIntoModal(workflowOrder[nextIdx]);
  }

  function isPlainPrimaryClick(event) {
    return (
      event.button === 0 &&
      !event.metaKey &&
      !event.ctrlKey &&
      !event.shiftKey &&
      !event.altKey
    );
  }

  // Bind every template card & crawlable .yml link for instant in-page YAML inspection
  tplItems.forEach((card) => {
    const filename = card.dataset.workflow;
    if (!filename) {
      return;
    }
    card.addEventListener("click", (event) => {
      const anchor = event.target.closest("a");
      if (anchor) {
        if (anchor.hasAttribute("data-workflow-link") && isPlainPrimaryClick(event)) {
          event.preventDefault();
          openWorkflowModal(filename);
        }
        return;
      }
      openWorkflowModal(filename);
    });
  });

  if (modalCopyYaml) {
    modalCopyYaml.addEventListener("click", () => {
      const wf = activeWorkflowKey ? getWorkflowData(activeWorkflowKey) : null;
      if (wf) {
        copyWithFeedback(modalCopyYaml, wf.yaml);
      }
    });
  }

  if (modalCopyCli && modalCliCode) {
    modalCopyCli.addEventListener("click", () => {
      copyWithFeedback(modalCopyCli, modalCliCode.dataset.rawCode || "");
    });
  }

  if (modalPrev) {
    modalPrev.addEventListener("click", () => stepWorkflow(-1));
  }
  if (modalNext) {
    modalNext.addEventListener("click", () => stepWorkflow(1));
  }

  // Fallback light-dismiss for browsers without native dialog[closedby="any"] (e.g. Safari)
  if (modal && !("closedBy" in HTMLDialogElement.prototype)) {
    const isOutside = (event) => {
      const rect = modal.getBoundingClientRect();
      return (
        event.clientX < rect.left ||
        event.clientX > rect.right ||
        event.clientY < rect.top ||
        event.clientY > rect.bottom
      );
    };
    let pressedOutside = false;
    modal.addEventListener("pointerdown", (event) => {
      pressedOutside = isOutside(event);
    });
    modal.addEventListener("pointerup", (event) => {
      if (pressedOutside && isOutside(event)) {
        modal.close();
      }
    });
  }

  if (modal) {
    modal.addEventListener("close", () => {
      modal.returnValue = "";
      activeWorkflowKey = null;
      syncWorkflowQuery(null);
    });
  }

  // Optional query deep-link (?workflow=illustrated_slides.yml)
  try {
    const wfParam = new URLSearchParams(location.search).get("workflow");
    if (wfParam && workflowOrder.includes(wfParam)) {
      openWorkflowModal(wfParam);
    }
  } catch {}
})();
