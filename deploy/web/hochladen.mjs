/**
 * Lädt dist/web (Webseite, Launcher-Download, signierte Updates) per SFTP auf den Strato-Webspace.
 *
 *   node hochladen.mjs                  – neue Version hochladen
 *   node hochladen.mjs --anzeigen [ordner] – Inhalt auf dem Server anzeigen
 *   node hochladen.mjs --archivieren [--behalten a,b] – alten Inhalt in einen gesperrten
 *        Archiv-Ordner verschieben (nichts wird gelöscht). `--behalten`: diese Einträge bleiben.
 *        Eine alte .htaccess landet als `htaccess.alt` im Archiv.
 *
 * Zugangsdaten stehen in .env.deploy neben diesem Skript (nie einchecken!).
 * Reihenfolge beim Hochladen: erst alle Dateien, zuletzt die Update-Liste – so sieht nie
 * jemand eine Liste, deren Dateien noch fehlen.
 */
import SftpClient from 'ssh2-sftp-client';
import dotenv from 'dotenv';
import { readdirSync, statSync } from 'node:fs';
import { join, posix, relative, sep } from 'node:path';

const hier = import.meta.dirname;
dotenv.config({ path: join(hier, '.env.deploy'), quiet: true });
const { SFTP_HOST, SFTP_PORT = '22', SFTP_USER, SFTP_PASSWORD, SFTP_REMOTE_DIR = '/' } = process.env;
if (!SFTP_HOST || !SFTP_USER || !SFTP_PASSWORD) {
  console.error('❌ deploy/web/.env.deploy fehlt oder ist unvollständig.');
  process.exit(1);
}

const quelle = join(hier, '..', '..', 'dist', 'web');
const ziel = SFTP_REMOTE_DIR.replace(/\/$/, '');
const sftp = new SftpClient();
const args = process.argv.slice(2);

function dateien(dir) {
  return readdirSync(dir).flatMap((name) => {
    const voll = join(dir, name);
    return statSync(voll).isDirectory() ? dateien(voll) : [voll];
  });
}

const heute = new Date().toISOString().slice(0, 10);
const archivName = `_archiv_${heute}`;
// Einträge, die beim Archivieren an Ort und Stelle bleiben (z. B. andere Anwendungen).
const behalten = new Set((args.includes('--behalten') ? args[args.indexOf('--behalten') + 1] ?? '' : '').split(',').filter(Boolean));

try {
  await sftp.connect({ host: SFTP_HOST, port: Number(SFTP_PORT), username: SFTP_USER, password: SFTP_PASSWORD, readyTimeout: 20000 });
  console.log(`✅ Verbunden mit ${SFTP_HOST}`);

  if (args.includes('--anzeigen')) {
    const ordner = args[args.indexOf('--anzeigen') + 1] ?? (ziel || '/');
    for (const e of (await sftp.list(ordner)).sort((a, b) => a.name.localeCompare(b.name))) {
      console.log(`  ${e.type === 'd' ? '📁' : '📄'} ${e.name}${e.type === 'd' ? '/' : `  (${e.size} B)`}`);
    }
  } else if (args.includes('--archivieren')) {
    const archiv = `${ziel}/${archivName}`;
    await sftp.mkdir(archiv, true).catch(() => {});
    // Von außen gesperrt – der alte Inhalt ist nur noch per SFTP erreichbar.
    const sperre = '<IfModule mod_authz_core.c>\n  Require all denied\n</IfModule>\n<IfModule !mod_authz_core.c>\n  Deny from all\n</IfModule>\n';
    await sftp.put(Buffer.from(sperre), `${archiv}/.htaccess`);
    let verschoben = 0;
    for (const e of await sftp.list(ziel || '/')) {
      if (e.name.startsWith('_archiv_') || behalten.has(e.name)) continue;
      // Die alte .htaccess nicht als .htaccess ablegen – sonst hebt sie die Sperre des Archivs auf.
      const neuerName = e.name === '.htaccess' ? 'htaccess.alt' : e.name;
      await sftp.rename(`${ziel}/${e.name}`, `${archiv}/${neuerName}`);
      console.log(`  → ${e.name}`);
      verschoben++;
    }
    console.log(`📦 ${verschoben} Einträge nach /${archivName}/ verschoben (gesperrt, nichts gelöscht).`);
  } else {
    const alle = dateien(quelle).map((voll) => ({ voll, rel: relative(quelle, voll).split(sep).join('/') }));
    const liste = alle.filter((d) => d.rel.endsWith('/manifest.signed'));
    const rest = alle.filter((d) => !d.rel.endsWith('/manifest.signed'));
    // Unveränderliche Dateien zuerst, Webseite danach, HTML ganz zum Schluss.
    const rang = (rel) => (rel.includes('/dateien/') ? 0 : rel.startsWith('downloads/') ? 1 : rel.endsWith('.html') ? 3 : 2);
    rest.sort((a, b) => rang(a.rel) - rang(b.rel));

    const ordner = new Set();
    let hoch = 0, gleich = 0, bytes = 0;
    for (const { voll, rel } of [...rest, ...liste]) {
      const entfernt = `${ziel}/${rel}`;
      const groesse = statSync(voll).size;
      // Dateien unter dateien/ heißen nach ihrem Inhalt: gleicher Name = gleicher Inhalt.
      if (rel.includes('/dateien/')) {
        const da = await sftp.stat(entfernt).catch(() => null);
        if (da && da.size === groesse) { gleich++; continue; }
      }
      const dir = posix.dirname(entfernt);
      if (!ordner.has(dir)) { await sftp.mkdir(dir, true).catch(() => {}); ordner.add(dir); }
      if (rel.endsWith('/manifest.signed')) {
        // Erst daneben ablegen, dann in einem Schritt austauschen.
        await sftp.fastPut(voll, `${entfernt}.neu`);
        await sftp.posixRename(`${entfernt}.neu`, entfernt).catch(async () => {
          await sftp.delete(entfernt).catch(() => {});
          await sftp.rename(`${entfernt}.neu`, entfernt);
        });
      } else {
        await sftp.fastPut(voll, entfernt);
      }
      hoch++;
      bytes += groesse;
      console.log(`  ↑ ${rel} (${(groesse / 1024).toFixed(0)} KB)`);
    }
    console.log(`🚀 ${hoch} Dateien (${(bytes / 1e6).toFixed(1)} MB) hochgeladen, ${gleich} waren schon da.`);
  }
} catch (err) {
  console.error('❌ Fehler:', err.message);
  process.exitCode = 1;
} finally {
  await sftp.end().catch(() => {});
}
