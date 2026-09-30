// Language switching and download links from the latest GitHub release.
(function () {
  const TEXT = window.SITE_TEXT;
  const STORAGE_KEY = "v360lab.site.lang";
  let lang = "en";
  let release = null;

  const format = (template, params) => template.replace(/\{(\w+)\}/g, (m, k) => (k in params ? params[k] : m));
  const t = (key, params = {}) => format((TEXT[lang] && TEXT[lang][key]) || TEXT.en[key] || key, params);

  function initialLanguage() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved && TEXT[saved]) return saved;
    } catch {}
    const preferred = (navigator.languages || [navigator.language || "en"]).map((l) => l.slice(0, 2).toLowerCase());
    return preferred.find((l) => TEXT[l]) || "en";
  }

  function renderRelease() {
    const info = document.getElementById("release-info");
    if (!release) {
      info.textContent = release === false ? t("releaseFallback") : t("releaseInfo");
      return;
    }
    const date = new Date(release.published_at).toLocaleDateString(lang);
    const tag = release.tag_name;
    info.textContent = t("releaseLatest", { tag, date });
    const badge = document.getElementById("version-badge");
    badge.textContent = t("versionBadge", { version: tag.replace(/^v/, ""), date });
    badge.hidden = false;
    document.getElementById("download-button").textContent = t("downloadVersion", { tag });
    document.getElementById("footer-version").textContent = ` · ${tag}`;
  }

  function apply(next) {
    lang = next;
    document.documentElement.lang = lang;
    document.title = t("title");
    document.querySelectorAll("[data-i18n]").forEach((el) => (el.textContent = t(el.dataset.i18n)));
    document.querySelectorAll("[data-i18n-html]").forEach((el) => (el.innerHTML = t(el.dataset.i18nHtml)));
    document.querySelectorAll("img[data-shot]").forEach((img) => {
      img.src = `assets/screenshots/${lang}/${img.dataset.shot}.jpg`;
      img.alt = img.closest("figure").querySelector("figcaption").textContent;
    });
    document.querySelectorAll(".lang-switch button").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.lang === lang)));
    renderRelease();
  }

  document.querySelectorAll(".lang-switch button").forEach((button) =>
    button.addEventListener("click", () => {
      try {
        localStorage.setItem(STORAGE_KEY, button.dataset.lang);
      } catch {}
      apply(button.dataset.lang);
    }),
  );
  apply(initialLanguage());

  const platforms = {
    android: [/\.apk$/i],
    windows: [/-setup\.exe$/i, /\.msi$/i],
    linux: [/\.AppImage$/i, /\.deb$/i, /\.rpm$/i],
  };
  fetch("https://api.github.com/repos/napo/V360Lab/releases/latest")
    .then((response) => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    })
    .then((data) => {
      release = data;
      for (const [platform, patterns] of Object.entries(platforms)) {
        const card = document.querySelector(`.download[data-platform="${platform}"]`);
        const assets = patterns.flatMap((p) => data.assets.filter((a) => p.test(a.name)));
        if (!card || assets.length === 0) continue;
        card.querySelector(".download-link").href = assets[0].browser_download_url;
        if (assets.length > 1) {
          const files = document.createElement("span");
          files.className = "files";
          for (const asset of assets) {
            const link = document.createElement("a");
            link.href = asset.browser_download_url;
            link.textContent = asset.name.split(".").pop();
            link.title = asset.name;
            files.appendChild(link);
          }
          card.appendChild(files);
        }
      }
      renderRelease();
    })
    .catch(() => {
      release = false;
      renderRelease();
    });
})();
