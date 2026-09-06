import { useEffect, useRef, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { drawerApi } from "../../services/drawerApi";
import type {
  BackupCenter,
  DiagnosticReport,
  HealthItem,
  Preferences,
  PreferencesPatch,
} from "../../types/drawer";

interface GlobalSettingsProps {
  preferences: Preferences | null;
  onPreferences: (preferences: Preferences) => void;
  onError: (message: string | null) => void;
  onMessage: (message: string | null) => void;
}

type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "current" | "error";
const AUTO_UPDATE_INTERVAL_MS = 24 * 60 * 60 * 1000;

export function GlobalSettings({ preferences, onPreferences, onError, onMessage }: GlobalSettingsProps) {
  const [backupCenter, setBackupCenter] = useState<BackupCenter | null>(null);
  const [health, setHealth] = useState<HealthItem[]>([]);
  const [diagnostic, setDiagnostic] = useState<DiagnosticReport | null>(null);
  const [appVersion, setAppVersion] = useState("…");
  const [updatePhase, setUpdatePhase] = useState<UpdatePhase>("idle");
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const [updateNotes, setUpdateNotes] = useState<string | null>(null);
  const [updateDate, setUpdateDate] = useState<string | null>(null);
  const pendingUpdate = useRef<Update | null>(null);
  const autoCheckStarted = useRef(false);

  async function refreshBackupCenter() {
    setBackupCenter(await drawerApi.getBackupCenter());
  }

  useEffect(() => {
    void getVersion().then(setAppVersion).catch(() => setAppVersion("desconocida"));
    void refreshBackupCenter().catch((reason) => onError(String(reason)));
    return () => {
      const update = pendingUpdate.current;
      pendingUpdate.current = null;
      if (update) void update.close();
    };
  }, []);

  useEffect(() => {
    if (!preferences?.checkUpdatesAutomatically || autoCheckStarted.current) return;
    if (Date.now() - preferences.lastUpdateCheck < AUTO_UPDATE_INTERVAL_MS) return;
    autoCheckStarted.current = true;
    void checkForApplicationUpdate(true);
  }, [preferences]);

  async function runAction(action: () => Promise<void>) {
    onError(null);
    onMessage(null);
    try {
      await action();
    } catch (reason) {
      onError(String(reason));
    }
  }

  async function updatePreference(patch: PreferencesPatch) {
    await runAction(async () => {
      onPreferences(await drawerApi.updatePreferences(patch));
      onMessage("Preferencia guardada.");
    });
  }

  async function checkForApplicationUpdate(silent = false) {
    if (["checking", "downloading", "installing"].includes(updatePhase)) return;
    if (pendingUpdate.current) {
      await installApplicationUpdate(pendingUpdate.current);
      return;
    }
    setUpdatePhase("checking");
    setUpdateProgress(null);
    setUpdateStatus("Buscando una versión nueva…");
    if (!silent) {
      onError(null);
      onMessage(null);
    }
    try {
      const update = await check({ timeout: 30_000 });
      if (!update) {
        setUpdatePhase("current");
        setUpdateStatus(`Desktop Organizer está actualizado. Versión instalada: ${appVersion}.`);
        onPreferences(await drawerApi.recordUpdateCheck(null));
        return;
      }
      pendingUpdate.current = update;
      setAvailableVersion(update.version);
      setUpdateNotes(update.body?.trim() || null);
      setUpdateDate(update.date ?? null);
      setUpdatePhase("available");
      setUpdateStatus(`Nueva versión disponible: ${update.version}.`);
      onPreferences(await drawerApi.recordUpdateCheck(update.version));
    } catch (reason) {
      setUpdatePhase("error");
      setUpdateStatus(`No se pudo buscar la actualización. La aplicación sigue funcionando sin Internet. ${String(reason)}`);
    }
  }

  async function installApplicationUpdate(update: Update) {
    let downloaded = 0;
    let total = 0;
    setUpdatePhase("downloading");
    setUpdateProgress(null);
    setUpdateStatus("Creando backup antes de actualizar…");
    try {
      await drawerApi.prepareUpdate();
      setUpdateStatus(`Descargando Desktop Organizer ${update.version}…`);
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          setUpdateProgress(total > 0 ? 0 : null);
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total > 0) {
            const percent = Math.min(100, Math.round((downloaded / total) * 100));
            setUpdateProgress(percent);
            setUpdateStatus(`Descargando Desktop Organizer ${update.version}… ${percent}%`);
          }
        } else {
          setUpdateProgress(100);
          setUpdatePhase("installing");
          setUpdateStatus("Paquete firmado descargado. Instalando y reiniciando la aplicación…");
        }
      }, { restartAfterInstall: true });
    } catch (reason) {
      pendingUpdate.current = null;
      setAvailableVersion(null);
      setUpdateProgress(null);
      setUpdatePhase("error");
      setUpdateStatus(`La actualización falló; la versión actual y tus datos permanecen intactos. ${String(reason)}`);
      await update.close().catch(() => undefined);
    }
  }

  const updateBusy = ["checking", "downloading", "installing"].includes(updatePhase);
  const updateButtonLabel = updatePhase === "checking"
    ? "Buscando…"
    : updatePhase === "downloading"
      ? updateProgress === null ? "Descargando…" : `Descargando ${updateProgress}%`
      : updatePhase === "installing"
        ? "Instalando…"
        : availableVersion
          ? `Instalar v${availableVersion}`
          : updatePhase === "error"
            ? "Reintentar"
            : "Buscar actualizaciones";

  return (
    <section id="global-settings" className="settings-center" aria-labelledby="global-settings-heading">
      <div className="settings-center-heading">
        <div>
          <p className="eyebrow">CONFIGURACIÓN GLOBAL</p>
          <h2 id="global-settings-heading">Centro de configuración</h2>
          <p>Una sola fuente para preferencias, recuperación, actualizaciones y diagnóstico.</p>
        </div>
        <button className="button button-ghost" onClick={() => void runAction(async () => drawerApi.openDataRoot())}>
          Abrir carpeta de datos
        </button>
      </div>

      <div className="settings-grid">
        <article className="settings-card">
          <p className="eyebrow">GENERAL</p>
          <h3>Inicio y Administrador</h3>
          <div className="preference-list">
            <PreferenceToggle label="Iniciar con Windows" detail="Inicio por usuario, sin permisos de administrador." checked={preferences?.startWithWindows ?? false} disabled={!preferences} onChange={(value) => updatePreference({ startWithWindows: value })} />
            <PreferenceToggle label="Inicio silencioso" detail="Restaura Cajones, Dock y Paneles sin abrir el Administrador." checked={preferences?.startSilently ?? true} disabled={!preferences} onChange={(value) => updatePreference({ startSilently: value })} />
            <PreferenceToggle label="Ocultar al minimizar" detail="La X siempre oculta; la salida real está en Tray → Salir." checked={preferences?.hideAdminOnMinimize ?? true} disabled={!preferences} onChange={(value) => updatePreference({ hideAdminOnMinimize: value })} />
          </div>
        </article>

        <article className="settings-card">
          <p className="eyebrow">APARIENCIA / RENDIMIENTO</p>
          <h3>Comportamiento global</h3>
          <PreferenceToggle label="Modo rendimiento" detail="Reduce sombras, blur y efectos costosos en todas las ventanas." checked={preferences?.performanceMode ?? false} disabled={!preferences} onChange={(value) => updatePreference({ performanceMode: value })} />
          <label className="setting-field">
            <span>Animaciones</span>
            <select value={preferences?.animationMode ?? "normal"} disabled={!preferences} onChange={(event) => void updatePreference({ animationMode: event.target.value as Preferences["animationMode"] })}>
              <option value="normal">Normales</option>
              <option value="reduced">Reducidas</option>
              <option value="disabled">Desactivadas</option>
            </select>
          </label>
          <p className="settings-note">La preferencia del sistema “reducir movimiento” también se respeta.</p>
        </article>

        <article className="settings-card">
          <p className="eyebrow">PANELES</p>
          <h3>Valores para Paneles nuevos</h3>
          <label className="setting-field">
            <span>Densidad predeterminada</span>
            <select value={preferences?.defaultPanelDensity ?? "normal"} disabled={!preferences} onChange={(event) => void updatePreference({ defaultPanelDensity: event.target.value as Preferences["defaultPanelDensity"] })}>
              <option value="compact">Compacta</option>
              <option value="normal">Normal</option>
              <option value="wide">Amplia</option>
            </select>
          </label>
          <label className="setting-field">
            <span>Tamaño de iconos</span>
            <select value={preferences?.defaultPanelIconMode ?? "auto"} disabled={!preferences} onChange={(event) => void updatePreference({ defaultPanelIconMode: event.target.value as Preferences["defaultPanelIconMode"] })}>
              <option value="auto">Automático</option>
              <option value="manual">Manual</option>
            </select>
          </label>
          <PreferenceToggle label="Imantado predeterminado" detail="A bordes y otros Paneles." checked={preferences?.defaultPanelSnapEnabled ?? true} disabled={!preferences} onChange={(value) => updatePreference({ defaultPanelSnapEnabled: value })} />
          <PreferenceToggle label="Crear bloqueados" detail="Bloquea posición y tamaño inicialmente." checked={preferences?.defaultPanelLocked ?? false} disabled={!preferences} onChange={(value) => updatePreference({ defaultPanelLocked: value })} />
        </article>

        <article className="settings-card settings-card-wide">
          <p className="eyebrow">BACKUP Y RECUPERACIÓN</p>
          <div className="settings-card-title-row">
            <div>
              <h3>{backupCenter?.saveValid ? "Configuración válida" : "Revisar configuración"}</h3>
              <p>{backupCenter?.backups.length ?? 0} backups · schema {backupCenter?.backups[0]?.schemaVersion ?? "—"}</p>
            </div>
            <button className="button button-primary" onClick={() => void runAction(async () => { await drawerApi.createBackup(); await refreshBackupCenter(); onMessage("Backup de configuración creado."); })}>
              Crear backup ahora
            </button>
          </div>
          <p className="settings-note">Incluye configuración, metadata, referencias, posiciones y orden. No incluye el contenido físico de Cajones.</p>
          <div className="settings-actions">
            <button className="button button-secondary" onClick={() => void runAction(async () => { const path = await drawerApi.exportConfiguration(); if (path) onMessage(`Configuración exportada en ${path}`); })}>Exportar configuración</button>
            <button className="button button-ghost" onClick={() => void runAction(async () => { const imported = await drawerApi.importConfiguration(); if (imported) { await refreshBackupCenter(); onMessage("Configuración importada y ventanas reconstruidas."); } })}>Importar configuración</button>
            <button className="button button-ghost" onClick={() => void runAction(async () => { await drawerApi.recoverFromDisk(); onMessage("Cajones recuperados desde el primer nivel del disco."); })}>Recuperar Cajones</button>
            <button className="button button-ghost" onClick={() => void runAction(async () => drawerApi.openDrawersRoot())}>Abrir Cajones</button>
          </div>
          <div className="backup-list">
            {backupCenter?.backups.length ? backupCenter.backups.map((backup) => (
              <div key={backup.fileName} className="backup-row">
                <span><strong>{new Date(backup.createdAt).toLocaleString()}</strong><small>schema {backup.schemaVersion ?? "?"} · {formatBytes(backup.size)} · {backup.valid ? "válido" : "inválido"}</small></span>
                <div>
                  <button className="button button-ghost" disabled={!backup.valid} onClick={() => void runAction(async () => { if (!window.confirm("Se creará un backup del estado actual antes de restaurar. ¿Continuar?")) return; await drawerApi.restoreBackup(backup.fileName); await refreshBackupCenter(); onMessage("Backup restaurado y ventanas reconstruidas."); })}>Restaurar</button>
                  <button className="button button-ghost" disabled={!backup.valid} onClick={() => void runAction(async () => { const path = await drawerApi.exportBackup(backup.fileName); if (path) onMessage(`Backup exportado en ${path}`); })}>Exportar</button>
                  <button className="button button-danger" onClick={() => void runAction(async () => { if (!window.confirm("¿Eliminar solamente esta copia de configuración?")) return; await drawerApi.deleteBackup(backup.fileName); await refreshBackupCenter(); onMessage("Backup eliminado."); })}>Eliminar</button>
                </div>
              </div>
            )) : <p className="settings-empty">Todavía no hay backups disponibles.</p>}
          </div>
        </article>

        <article className="settings-card">
          <p className="eyebrow">ACTUALIZACIONES</p>
          <h3>Canal Stable</h3>
          <p>{updateStatus ?? `Versión instalada: ${appVersion}`}</p>
          {updateDate && <small>Publicada: {new Date(updateDate).toLocaleString()}</small>}
          {updateNotes && <p className="update-notes">{updateNotes}</p>}
          <PreferenceToggle label="Buscar automáticamente" detail="Como máximo una vez cada 24 horas." checked={preferences?.checkUpdatesAutomatically ?? true} disabled={!preferences} onChange={(value) => updatePreference({ checkUpdatesAutomatically: value })} />
          <button className="button button-secondary" disabled={updateBusy} onClick={() => void checkForApplicationUpdate(false)}>{updateButtonLabel}</button>
          {updateBusy && <div className={`update-progress-track ${updateProgress === null ? "is-indeterminate" : ""}`} role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={updateProgress ?? undefined}><div className="update-progress-value" style={updateProgress === null ? undefined : { width: `${updateProgress}%` }} /></div>}
          <p className="settings-note">Los paquetes se validan con la firma pública configurada. Antes de instalar se crea un backup.</p>
        </article>

        <article className="settings-card">
          <p className="eyebrow">DIAGNÓSTICO</p>
          <h3>Estado local</h3>
          {health.length > 0 && <div className="health-list">{health.map((item) => <span key={item.label} className={item.ok ? "is-ok" : "is-error"}><strong>{item.label}: {item.ok ? "OK" : "Problema"}</strong><small>{item.detail}</small></span>)}</div>}
          {diagnostic && <p>v{diagnostic.appVersion} · schema {diagnostic.schemaVersion} · {diagnostic.drawerCount} Cajones · {diagnostic.dockItemCount} Dock · {diagnostic.panelCount} Paneles</p>}
          <div className="settings-actions">
            <button className="button button-secondary" onClick={() => void runAction(async () => { const result = await drawerApi.checkHealth(); setHealth(result); onMessage("Comprobación de estado terminada."); })}>Comprobar estado</button>
            <button className="button button-ghost" onClick={() => void runAction(async () => { const report = await drawerApi.getDiagnosticReport(); setDiagnostic(report); await navigator.clipboard.writeText(report.text); onMessage("Diagnóstico sin secretos copiado."); })}>Copiar diagnóstico</button>
            <button className="button button-ghost" onClick={() => void runAction(async () => drawerApi.openLogsRoot())}>Abrir logs</button>
            <button className="button button-ghost" onClick={() => void runAction(async () => { if (!window.confirm("¿Limpiar solamente los logs de Desktop Organizer?")) return; await drawerApi.clearLogs(); onMessage("Logs eliminados."); })}>Limpiar logs</button>
          </div>
          <p className="settings-note">No se incluyen tokens, contenido de documentos ni nombres de archivos personales.</p>
        </article>

        <article className="settings-card settings-card-danger">
          <p className="eyebrow">RECUPERACIÓN VISUAL</p>
          <h3>Restablecer configuración visual</h3>
          <p>Restablece apariencia y posiciones. Conserva archivos, accesos y referencias.</p>
          <button className="button button-danger" onClick={() => void runAction(async () => { if (!window.confirm("Se creará un backup. Tus archivos no se borrarán. ¿Restablecer apariencia y posiciones?")) return; await drawerApi.resetVisualConfiguration(); await refreshBackupCenter(); onMessage("Configuración visual restablecida; los archivos permanecen intactos."); })}>Restablecer configuración visual</button>
        </article>
      </div>
    </section>
  );
}

interface PreferenceToggleProps {
  label: string;
  detail: string;
  checked: boolean;
  disabled: boolean;
  onChange: (value: boolean) => Promise<void>;
}

function PreferenceToggle({ label, detail, checked, disabled, onChange }: PreferenceToggleProps) {
  return (
    <label className="preference-row">
      <span><strong>{label}</strong><small>{detail}</small></span>
      <input type="checkbox" checked={checked} disabled={disabled} onChange={(event) => void onChange(event.target.checked)} />
    </label>
  );
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
