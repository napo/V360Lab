// Fills the download cards with the assets of the latest GitHub release.
(async function () {
  const info = document.getElementById("release-info");
  const platforms = {
    android: [/\.apk$/i],
    windows: [/-setup\.exe$/i, /\.msi$/i],
    linux: [/\.AppImage$/i, /\.deb$/i, /\.rpm$/i],
  };
  try {
    const response = await fetch("https://api.github.com/repos/napo/V360Lab/releases/latest");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const release = await response.json();
    info.textContent = `Latest release: ${release.tag_name} (${new Date(release.published_at).toLocaleDateString()})`;
    for (const [platform, patterns] of Object.entries(platforms)) {
      const card = document.querySelector(`.download[data-platform="${platform}"]`);
      const assets = patterns.flatMap((p) => release.assets.filter((a) => p.test(a.name)));
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
  } catch {
    info.textContent = "See all builds on the GitHub releases page.";
  }
})();
