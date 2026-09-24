// Holt Version und Neuigkeiten (schreibt das Release-Werkzeug nach version.json)
// und hebt den Download für das eigene Betriebssystem hervor.
(async () => {
  const mac = /Mac/i.test(navigator.platform || navigator.userAgent);
  document.getElementById(mac ? 'dl-mac' : 'dl-windows').classList.add('empfohlen');

  try {
    const antwort = await fetch('version.json', { cache: 'no-store' });
    if (!antwort.ok) throw new Error(antwort.status);
    const info = await antwort.json();
    const datum = (info.date || '').split('-').reverse().join('.');
    document.getElementById('version').textContent = `Aktuelle Version ${info.version} · ${datum}`;

    const windows = info.downloads && info.downloads.windows;
    if (windows) {
      document.getElementById('dl-windows').href = windows.file;
      const mb = (windows.size / 1e6).toFixed(0);
      document.getElementById('dl-windows-info').textContent = `Windows 10 und 11 · ${mb} MB · kostenlos`;
    }
    const macos = info.downloads && info.downloads.macos;
    if (macos) {
      const knopf = document.getElementById('dl-mac');
      const link = document.createElement('a');
      link.className = 'dl dl--an' + (mac ? ' empfohlen' : '');
      link.id = 'dl-mac';
      link.href = macos.file;
      link.innerHTML = knopf.innerHTML.replace('Bald verfügbar', `Apple Silicon und Intel · ${(macos.size / 1e6).toFixed(0)} MB`);
      knopf.replaceWith(link);
    }

    document.getElementById('neues-kopf').textContent = `Version ${info.version} vom ${datum}`;
    const liste = document.getElementById('neues-liste');
    for (const zeile of (info.news && info.news.length ? info.news : ['Fehlerbehebungen und Verbesserungen.'])) {
      const eintrag = document.createElement('li');
      eintrag.textContent = zeile;
      liste.appendChild(eintrag);
    }
  } catch {
    document.getElementById('neues-kopf').textContent = 'Neuigkeiten gibt es im Launcher.';
  }
})();
