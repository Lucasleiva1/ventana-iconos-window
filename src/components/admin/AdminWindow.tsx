import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useAppState } from "../../hooks/useAppState";
import { drawerApi } from "../../services/drawerApi";
import type { Drawer, MonitorInfo, Preferences, RecoveryStatus } from "../../types/drawer";
import { DrawerCard } from "./DrawerCard";
import { DockSettings } from "./DockSettings";

type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "current" | "error";

export function AdminWindow() {
  const { state, error } = useAppState();
  const [newDrawerName, setNewDrawerName] = useState("");
  const [creating, setCreating] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [recovery, setRecovery] = useState<RecoveryStatus | null>(null);
  const [preferences, setPreferences] = useState<Preferences | null>(null);
  const [appVersion, setAppVersion] = useState("…");
  const [updatePhase, setUpdatePhase] = useState<UpdatePhase>("idle");
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const pendingUpdate = useRef<Update | null>(null);
  const newDrawerInput = useRef<HTMLInputElement>(null);
  const generalSettings = useRef<HTMLElement>(null);

  useEffect(() => {
    void getVersion().then(setAppVersion).catch(() => setAppVersion("desconocida"));
    void drawerApi.getMonitors().then(setMonitors).catch((reason) => {
      setActionError(`No se pudo leer la información de monitores: ${String(reason)}`);
    });
  }, []);

  useEffect(() => () => {
    const update = pendingUpdate.current;
    pendingUpdate.current = null;
    if (update) void update.close();
  }, []);

  useEffect(() => {
    void drawerApi.getPreferences().then(setPreferences).catch((reason) => {
      setActionError(`No se pudo consultar el inicio con Windows: ${String(reason)}`);
    });
    const listeners = Promise.all([
      drawerApi.onFocusNewDrawer(() => {
        newDrawerInput.current?.scrollIntoView({ behavior: "smooth", block: "center" });
        newDrawerInput.current?.focus();
      }),
      drawerApi.onOpenSettings(() => {
        generalSettings.current?.scrollIntoView({ behavior: "smooth", block: "center" });
      }),
    ]);
    return () => {
      void listeners.then((unlisten) => unlisten.forEach((stop) => stop()));
    };
  }, []);

  useEffect(() => {
    void drawerApi.getRecoveryStatus().then(setRecovery).catch((reason) => {
      setActionError(`No se pudo verificar la recuperación: ${String(reason)}`);
    });
  }, [state]);

  const drawers = useMemo(
    () => [...(state?.drawers ?? [])].sort((a, b) => a.createdAt - b.createdAt),
    [state],
  );

  async function createDrawer(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = newDrawerName.trim();
    if (!name) return;

    setCreating(true);
    setActionError(null);
    try {
      await drawerApi.create(name);
      setNewDrawerName("");
    } catch (reason) {
      setActionError(String(reason));
    } finally {
      setCreating(false);
    }
  }

  async function runAction(action: () => Promise<unknown>) {
    setActionError(null);
    setActionMessage(null);
    try {
      await action();
    } catch (reason) {
      setActionError(String(reason));
    }
  }

  async function exportConfiguration() {
    await runAction(async () => {
      const path = await drawerApi.exportConfiguration();
      if (path) setActionMessage(`Configuración exportada en ${path}`);
    });
  }

  async function importConfiguration() {
    await runAction(async () => {
      const imported = await drawerApi.importConfiguration();
      if (imported) setActionMessage("Configuración importada y combinada sin borrar datos existentes.");
    });
  }

  async function recoverFromDisk() {
    await runAction(async () => {
      await drawerApi.recoverFromDisk();
      setActionMessage("Cajones recuperados desde sus carpetas físicas.");
    });
  }

  async function startFresh() {
    await runAction(async () => {
      await drawerApi.startFresh();
      setActionMessage("Se creó una configuración nueva. Las carpetas existentes no se borraron.");
    });
  }

  async function updatePreference(patch: Parameters<typeof drawerApi.updatePreferences>[0]) {
    await runAction(async () => {
      setPreferences(await drawerApi.updatePreferences(patch));
      setActionMessage("Preferencia guardada.");
    });
  }

  async function copyStoragePath(path: string | undefined, label: string) {
    if (!path) return;
    await runAction(async () => {
      await navigator.clipboard.writeText(path);
      setActionMessage(`Ruta de ${label} copiada al portapapeles.`);
    });
  }

  async function checkForApplicationUpdate() {
    if (updatePhase === "checking" || updatePhase === "downloading" || updatePhase === "installing") return;
    if (pendingUpdate.current) {
      await installApplicationUpdate(pendingUpdate.current);
      return;
    }

    setUpdatePhase("checking");
    setUpdateProgress(null);
    setActionError(null);
    setActionMessage(null);
    setUpdateStatus("Buscando una versión nueva…");

    try {
      const update = await check({ timeout: 30_000 });
      if (!update) {
        setUpdatePhase("current");
        setUpdateStatus(`Ya está actualizado. Versión instalada: ${appVersion}.`);
        return;
      }

      pendingUpdate.current = update;
      setAvailableVersion(update.version);
      setUpdatePhase("available");
      setUpdateStatus(`Versión ${update.version} disponible. Pulsá Instalar para actualizar y reiniciar.`);
    } catch (reason) {
      setUpdatePhase("error");
      setUpdateStatus(`No se pudo buscar la actualización. Pulsá Reintentar. ${String(reason)}`);
    }
  }

  async function installApplicationUpdate(update: Update) {
    let downloaded = 0;
    let total = 0;
    setUpdatePhase("downloading");
    setUpdateProgress(null);
    setUpdateStatus(`Descargando Desktop Organizer ${update.version}…`);

    try {
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
          setUpdateStatus("Descarga terminada. Instalando y reiniciando…");
        }
      }, { restartAfterInstall: true });
    } catch (reason) {
      pendingUpdate.current = null;
      setAvailableVersion(null);
      setUpdateProgress(null);
      setUpdatePhase("error");
      setUpdateStatus(`No se pudo instalar la actualización. Pulsá Reintentar. ${String(reason)}`);
      await update.close().catch(() => undefined);
    }
  }

  const updateBusy = updatePhase === "checking" || updatePhase === "downloading" || updatePhase === "installing";
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
    <main className="admin-shell">
      <header className="admin-header">
        <div>
          <p className="eyebrow">ORGANIZADOR VISUAL</p>
          <h1>Desktop Organizer</h1>
          <p className="admin-subtitle">
            Administrá Cajones y Dock respaldados por carpetas reales y separadas en Documentos.
          </p>
        </div>
        <div className="monitor-summary" aria-label="Monitores detectados">
          <span className="monitor-count">{monitors.length || "—"}</span>
          <span>{monitors.length === 1 ? "monitor" : "monitores"}</span>
        </div>
      </header>

      <section className="backup-panel" aria-label="Guardado y recuperación">
        <div>
          <strong>Guardado seguro</strong>
          <span title={recovery?.storage.masterSave}>
            {recovery?.storage.root ?? "Verificando ubicación…"}
          </span>
        </div>
        <div className="backup-actions">
          <button className="button button-ghost" onClick={() => void runAction(() => drawerApi.openDrawersRoot())}>
            Abrir carpeta Cajones
          </button>
          <button className="button button-ghost" disabled={!recovery} onClick={() => void copyStoragePath(recovery?.storage.drawers, "Cajones")}>
            Copiar ruta Cajones
          </button>
          <button className="button button-ghost" onClick={() => void runAction(() => drawerApi.openDockRoot())}>
            Abrir carpeta Dock
          </button>
          <button className="button button-ghost" disabled={!recovery} onClick={() => void copyStoragePath(recovery?.storage.dock, "Dock")}>
            Copiar ruta Dock
          </button>
          <button className="button button-secondary" onClick={() => void exportConfiguration()}>
            Exportar configuración
          </button>
          <button className="button button-ghost" onClick={() => void importConfiguration()}>
            Importar configuración
          </button>
          <button className="button button-ghost" onClick={() => void recoverFromDisk()}>
            Recuperar desde disco
          </button>
        </div>
      </section>

      {recovery?.notice && <div className="success-banner" role="status">{recovery.notice}</div>}

      <section ref={generalSettings} id="general-settings" className="general-settings-panel" aria-labelledby="general-settings-heading">
        <div>
          <p className="eyebrow">CONFIGURACIÓN</p>
          <h2 id="general-settings-heading">General</h2>
          <p>El inicio automático es por usuario de Windows y no requiere permisos de administrador.</p>
        </div>
        <div className="preference-list">
          <label className="preference-row">
            <span><strong>Iniciar con Windows</strong><small>Restaura cajones y el icono del área de notificación.</small></span>
            <input type="checkbox" checked={preferences?.startWithWindows ?? false} disabled={!preferences} onChange={(event) => void updatePreference({ startWithWindows: event.target.checked })} />
          </label>
          <label className="preference-row">
            <span><strong>Ocultar administrador al minimizar</strong><small>Los cajones siguen funcionando.</small></span>
            <input type="checkbox" checked={preferences?.hideAdminOnMinimize ?? true} disabled={!preferences} onChange={(event) => void updatePreference({ hideAdminOnMinimize: event.target.checked })} />
          </label>
          <label className="preference-row">
            <span><strong>Inicio silencioso</strong><small>Al iniciar con Windows, no abre el administrador.</small></span>
            <input type="checkbox" checked={preferences?.startSilently ?? true} disabled={!preferences} onChange={(event) => void updatePreference({ startSilently: event.target.checked })} />
          </label>
          <div className="preference-row update-preference-row">
            <span>
              <strong>Actualizaciones</strong>
              <small>{updateStatus ?? `Versión instalada: ${appVersion}`}</small>
            </span>
            <div className="update-control">
              <button
                type="button"
                className="button button-secondary"
                disabled={updateBusy}
                onClick={() => void checkForApplicationUpdate()}
              >
                {updateButtonLabel}
              </button>
              {updateBusy && (
                <div
                  className={`update-progress-track ${updateProgress === null ? "is-indeterminate" : ""}`}
                  role="progressbar"
                  aria-label={updateStatus ?? "Actualizando"}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={updateProgress ?? undefined}
                >
                  <div
                    className="update-progress-value"
                    style={updateProgress === null ? undefined : { width: `${updateProgress}%` }}
                  />
                </div>
              )}
            </div>
          </div>
        </div>
      </section>

      <DockSettings monitors={monitors} onError={setActionError} onMessage={setActionMessage} />

      {recovery && recovery.recoverableDrawers > 0 && (
        <section className="recovery-panel" aria-labelledby="recovery-heading">
          <div>
            <p className="eyebrow">RECUPERACIÓN SEGURA</p>
            <h2 id="recovery-heading">
              {recovery.recoverableDrawers} {recovery.recoverableDrawers === 1 ? "carpeta de cajón disponible" : "carpetas de cajón disponibles"}
            </h2>
            <p>
              El contenido físico está intacto en <code>{recovery.storage.drawers}</code>.
              Podés recuperar los cajones o importar una copia de configuración.
            </p>
          </div>
          <div className="recovery-actions">
            <button className="button button-primary" onClick={() => void recoverFromDisk()}>
              Recuperar desde disco
            </button>
            <button className="button button-secondary" onClick={() => void importConfiguration()}>
              Buscar copia
            </button>
            {!recovery.masterSaveExists && (
              <button className="button button-ghost" onClick={() => void startFresh()}>
                Empezar vacío
              </button>
            )}
          </div>
        </section>
      )}

      <section className="create-panel" aria-labelledby="create-heading">
        <div>
          <h2 id="create-heading">Nuevo cajón</h2>
          <p>Elegí un nombre claro. Podrás cambiarlo cuando quieras.</p>
        </div>
        <form className="create-form" onSubmit={createDrawer}>
          <label className="sr-only" htmlFor="drawer-name">
            Nombre del cajón
          </label>
          <input
            id="drawer-name"
            ref={newDrawerInput}
            value={newDrawerName}
            onChange={(event) => setNewDrawerName(event.target.value)}
            placeholder="Nombre del cajón"
            maxLength={80}
            autoComplete="off"
          />
          <button className="button button-primary" disabled={creating || !newDrawerName.trim()}>
            {creating ? "Creando…" : "+ Nuevo cajón"}
          </button>
        </form>
      </section>

      {(error || actionError) && (
        <div className="error-banner" role="alert">
          {error || actionError}
        </div>
      )}
      {actionMessage && <div className="success-banner" role="status">{actionMessage}</div>}

      <section className="drawers-section" aria-labelledby="drawers-heading">
        <div className="section-heading">
          <div>
            <p className="eyebrow">CAJONES</p>
            <h2 id="drawers-heading">
              {drawers.length} {drawers.length === 1 ? "cajón" : "cajones"}
            </h2>
          </div>
          <div className="bulk-actions">
            <button
              className="button button-secondary"
              disabled={!drawers.length}
              onClick={() => void runAction(() => drawerApi.setAllHidden(false))}
            >
              Mostrar todos
            </button>
            <button
              className="button button-ghost"
              disabled={!drawers.length}
              onClick={() => void runAction(() => drawerApi.setAllHidden(true))}
            >
              Ocultar todos
            </button>
          </div>
        </div>

        {!state ? (
          <div className="empty-admin">Cargando cajones…</div>
        ) : drawers.length === 0 ? (
          <div className="empty-admin">
            <div className="empty-symbol" aria-hidden="true">◇</div>
            <strong>Todavía no hay cajones</strong>
            <span>Creá el primero para empezar a ordenar tu escritorio.</span>
          </div>
        ) : (
          <div className="drawer-list">
            {drawers.map((drawer: Drawer) => (
              <DrawerCard key={drawer.id} drawer={drawer} onError={setActionError} />
            ))}
          </div>
        )}
      </section>
    </main>
  );
}
