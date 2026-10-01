// Site translations. English is also written in index.html (no-JS fallback).
window.SITE_TEXT = {
  en: {
    title: "V360Lab — Open tools for Garmin VIRB 360",
    tagline:
      "Control your Garmin VIRB 360 from your phone or computer, download media and FIT telemetry, and prepare 360° data for geospatial and computer-vision work. Local only, no cloud.",
    download: "Download",
    downloadVersion: "Download {tag}",
    source: "Source on GitHub",
    releaseInfo: "Latest release from GitHub.",
    releaseLatest: "Latest release: {tag} ({date})",
    releaseFallback: "See all builds on the GitHub releases page.",
    versionBadge: "Version {version} · {date}",
    androidFiles: "Signed APK",
    windowsFiles: "Installer (.exe / .msi)",
    downloadNote:
      "All builds are produced by GitHub Actions from the public source code. Windows builds are not code-signed yet, so SmartScreen may warn on first launch. On Android, allow installation from your browser or file manager.",
    featuresTitle: "What it does",
    f1Title: "Find and connect",
    f1Text:
      "One button: tries the last camera, then the camera's own Wi-Fi, then scans your local network. The camera is usually found in 2–3 seconds, with each step shown as it happens.",
    f2Title: "Live preview and capture",
    f2Text:
      "See what the camera sees and look around the 360° view with your finger, switch between video and photo, choose 360°, front, rear or RAW lens mode, single, burst or interval photos, and start or stop with one big shutter button.",
    f3Title: "Media and telemetry",
    f3Text:
      "Browse the card with thumbnails, download media together with FIT telemetry and the original camera metadata, and delete files with verification.",
    f4Title: "All camera settings",
    f4Text:
      "Every setting the camera reports (white balance, ISO, exposure, microphone, GPS…) can be changed, with readable names in English and Italian. Save Wi-Fi networks on the camera, or make it beep to find it.",
    f5Title: "Local and private",
    f5Text:
      "V360Lab talks only to the camera on your network over its HTTP API. No Garmin account, no cloud service, no data leaves your devices.",
    f6Title: "Open source",
    f6Text:
      "Built with Rust, Tauri, React and TypeScript, licensed under the GNU AGPL-3.0, and tested against a real VIRB 360 (firmware 4.20).",
    screenshotsTitle: "Screenshots",
    shotVideo: "Video capture",
    shotPhoto: "Interval photos",
    shotMedia: "Media library",
    shotSettings: "Camera settings",
    roadmapTitle: "Roadmap",
    r1: "<strong>Camera toolkit</strong> — connection, live preview, capture control, media browser, media and FIT download",
    r2: "<strong>Telemetry</strong> — FIT parsing, timeline, GPS track, synchronized video and map",
    r3: "<strong>Computer vision</strong> — frame extraction, OpenCV, YOLO detection, SAM segmentation",
    r4: "<strong>Geospatial outputs</strong> — georeferenced detections, GeoJSON / GeoParquet, MapLibre",
    r5: "<strong>3D</strong> — depth estimation, photogrammetry, SfM, point clouds",
    disclaimer:
      "V360Lab is an independent open-source project and is not affiliated with, sponsored by, or endorsed by Garmin. Garmin and VIRB are trademarks of Garmin Ltd. or its subsidiaries.",
    allReleases: "All releases",
  },
  it: {
    title: "V360Lab — Strumenti open source per Garmin VIRB 360",
    tagline:
      "Controlla la tua Garmin VIRB 360 dal telefono o dal computer, scarica media e telemetria FIT e prepara i dati 360° per analisi geospaziali e di computer vision. Tutto in locale, senza cloud.",
    download: "Scarica",
    downloadVersion: "Scarica {tag}",
    source: "Codice su GitHub",
    releaseInfo: "Ultima release da GitHub.",
    releaseLatest: "Ultima release: {tag} ({date})",
    releaseFallback: "Trovi tutte le build nella pagina delle release su GitHub.",
    versionBadge: "Versione {version} · {date}",
    androidFiles: "APK firmato",
    windowsFiles: "Installer (.exe / .msi)",
    downloadNote:
      "Tutte le build sono prodotte da GitHub Actions a partire dal codice sorgente pubblico. Le build per Windows non sono ancora firmate, quindi al primo avvio SmartScreen potrebbe mostrare un avviso. Su Android, consenti l'installazione dal browser o dal file manager.",
    featuresTitle: "Cosa fa",
    f1Title: "Trova e connetti",
    f1Text:
      "Un solo pulsante: prova l'ultima camera usata, poi il Wi-Fi della camera, poi cerca nella rete locale. Di solito la camera viene trovata in 2–3 secondi, e ogni passaggio è mostrato mentre avviene.",
    f2Title: "Anteprima dal vivo e scatto",
    f2Text:
      "Guarda cosa vede la camera e muoviti nella vista a 360° con un dito, passa da video a foto, scegli obiettivo 360°, anteriore, posteriore o RAW, foto singole, a raffica o a intervalli, e avvia o ferma con un grande pulsante di scatto.",
    f3Title: "Media e telemetria",
    f3Text:
      "Sfoglia la scheda con le miniature, scarica i media insieme alla telemetria FIT e ai metadati originali della camera, ed elimina i file con verifica.",
    f4Title: "Tutte le impostazioni",
    f4Text:
      "Ogni impostazione riportata dalla camera (bilanciamento del bianco, ISO, esposizione, microfono, GPS…) si può modificare, con nomi leggibili in italiano e inglese. Salva reti Wi-Fi sulla camera, o falla suonare per ritrovarla.",
    f5Title: "Locale e privato",
    f5Text:
      "V360Lab comunica solo con la camera nella tua rete, tramite la sua API HTTP. Nessun account Garmin, nessun servizio cloud: i dati non lasciano i tuoi dispositivi.",
    f6Title: "Open source",
    f6Text:
      "Realizzato con Rust, Tauri, React e TypeScript, con licenza GNU AGPL-3.0, e provato su una vera VIRB 360 (firmware 4.20).",
    screenshotsTitle: "Screenshot",
    shotVideo: "Registrazione video",
    shotPhoto: "Foto a intervalli",
    shotMedia: "Libreria media",
    shotSettings: "Impostazioni della camera",
    roadmapTitle: "Roadmap",
    r1: "<strong>Toolkit per la camera</strong> — connessione, anteprima dal vivo, controllo dello scatto, libreria media, download di media e FIT",
    r2: "<strong>Telemetria</strong> — lettura dei FIT, timeline, traccia GPS, video e mappa sincronizzati",
    r3: "<strong>Computer vision</strong> — estrazione dei fotogrammi, OpenCV, rilevamento con YOLO, segmentazione con SAM",
    r4: "<strong>Dati geospaziali</strong> — rilevamenti georeferenziati, GeoJSON / GeoParquet, MapLibre",
    r5: "<strong>3D</strong> — stima della profondità, fotogrammetria, SfM, nuvole di punti",
    disclaimer:
      "V360Lab è un progetto open source indipendente e non è affiliato, sponsorizzato o approvato da Garmin. Garmin e VIRB sono marchi di Garmin Ltd. o delle sue controllate.",
    allReleases: "Tutte le release",
  },
};
