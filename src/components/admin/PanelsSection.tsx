import { useState, type FormEvent } from "react";
import { drawerApi } from "../../services/drawerApi";
import { panelApi } from "../../services/panelApi";
import type { MonitorInfo, Preferences, PreferencesPatch } from "../../types/drawer";
import type { Panel, PanelAlignment } from "../../types/panel";

const ALIGNMENTS: Array<{ value: PanelAlignment; label: string }> = [
  { value: "left", label: "Alinear izquierda" },
  { value: "top", label: "Alinear arriba" },
  { value: "distributeHorizontally", label: "Distribuir en fila" },
  { value: "distributeVertically", label: "Distribuir en columna" },
];

interface PanelsSectionProps {
  panels: Panel[];
  monitors: MonitorInfo[];
  preferences: Preferences | null;
  onPreferences: (preferences: Preferences) => void;
  onError: (message: string | null) => void;
  onMessage: (message: string) => void;
}

export function PanelsSection({
  panels,
  monitors,
  preferences,
  onPreferences,
  onError,
  onMessage,
}: PanelsSectionProps) {
  const [newPanelName, setNewPanelName] = useState("");
  const [creating, setCreating] = useState(false);

  async function run(action: () => Promise<unknown>, message?: string) {
    onError(null);
    try {
      await action();
      if (message) onMessage(message);
    } catch (reason) {
      onError(String(reason));
    }
  }

  async function updatePanelDefaults(patch: PreferencesPatch) {
    await run(async () => {
      onPreferences(await drawerApi.updatePreferences(patch));
    }, "Preferencia para nuevos paneles guardada.");
  }

  async function createPanel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = newPanelName.trim();
    if (!name) return;
    setCreating(true);
    await run(async () => {
      await panelApi.create(name);
      setNewPanelName("");
    }, `Panel “${name}” creado.`);
    setCreating(false);
  }

  function monitorLabel(panel: Panel) {
    const monitor = monitors.find((value) => value.id === panel.monitorId);
    if (!monitor) return "Monitor principal";
    return `${monitor.name}${monitor.primary ? " (principal)" : ""}`;
  }

  const sorted = [...panels].sort((a, b) => a.createdAt - b.createdAt);

  return (
    <section className="panels-section" aria-labelledby="panels-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">PANELES</p>
          <h2 id="panels-heading">
            {sorted.length} {sorted.length === 1 ? "panel" : "paneles"}
          </h2>
          <p className="panels-intro">
            Los Paneles quedan visibles sobre el escritorio y guardan accesos.
            Lo que arrastres a un Panel nunca se mueve de su lugar.
          </p>
        </div>
        <div className="bulk-actions">
          <button
            className="button button-secondary"
            disabled={!sorted.length}
            onClick={() => void run(() => panelApi.setAllHidden(false), "Paneles visibles.")}
          >
            Mostrar todos
          </button>
          <button
            className="button button-ghost"
            disabled={!sorted.length}
            onClick={() => void run(() => panelApi.setAllHidden(true), "Paneles ocultos.")}
          >
            Ocultar todos
          </button>
        </div>
      </div>

      <div className="panel-defaults">
        <div>
          <p className="eyebrow">NUEVOS PANELES</p>
          <strong>Valores predeterminados</strong>
          <small>Se aplican cuando creás un panel nuevo.</small>
        </div>
        <label className="setting-field">
          <span>Densidad</span>
          <select
            value={preferences?.defaultPanelDensity ?? "normal"}
            disabled={!preferences}
            onChange={(event) => void updatePanelDefaults({
              defaultPanelDensity: event.target.value as Preferences["defaultPanelDensity"],
            })}
          >
            <option value="compact">Compacta</option>
            <option value="normal">Normal</option>
            <option value="wide">Amplia</option>
          </select>
        </label>
        <label className="setting-field">
          <span>Iconos</span>
          <select
            value={preferences?.defaultPanelIconMode ?? "auto"}
            disabled={!preferences}
            onChange={(event) => void updatePanelDefaults({
              defaultPanelIconMode: event.target.value as Preferences["defaultPanelIconMode"],
            })}
          >
            <option value="auto">Automáticos</option>
            <option value="manual">Manuales</option>
          </select>
        </label>
        <label className="compact-toggle">
          <input
            type="checkbox"
            checked={preferences?.defaultPanelSnapEnabled ?? true}
            disabled={!preferences}
            onChange={(event) => void updatePanelDefaults({
              defaultPanelSnapEnabled: event.target.checked,
            })}
          />
          <span>Imantado</span>
        </label>
        <label className="compact-toggle">
          <input
            type="checkbox"
            checked={preferences?.defaultPanelLocked ?? false}
            disabled={!preferences}
            onChange={(event) => void updatePanelDefaults({
              defaultPanelLocked: event.target.checked,
            })}
          />
          <span>Bloqueado</span>
        </label>
      </div>

      {sorted.length > 1 && (
        <div className="panel-align-bar">
          <span>Ordenar en pantalla</span>
          {ALIGNMENTS.map((option) => (
            <button
              key={option.value}
              className="button button-ghost"
              onClick={() => void run(() => panelApi.align(option.value), "Paneles reubicados.")}
            >
              {option.label}
            </button>
          ))}
        </div>
      )}

      <form className="create-form" onSubmit={createPanel}>
        <label className="sr-only" htmlFor="panel-name">Nombre del panel</label>
        <input
          id="panel-name"
          value={newPanelName}
          onChange={(event) => setNewPanelName(event.target.value)}
          placeholder="Nombre del panel"
          maxLength={60}
          autoComplete="off"
        />
        <button className="button button-primary" disabled={creating || !newPanelName.trim()}>
          {creating ? "Creando…" : "+ Nuevo panel"}
        </button>
      </form>

      {sorted.length === 0 ? (
        <div className="empty-admin">
          <div className="empty-symbol" aria-hidden="true">◇</div>
          <strong>Todavía no hay paneles</strong>
          <span>Creá uno para tener tus accesos siempre a la vista.</span>
        </div>
      ) : (
        <div className="panel-list">
          {sorted.map((panel) => (
            <article className="panel-card" key={panel.id}>
              <div className="panel-card-main">
                <span className="panel-card-dot" style={{ background: panel.color }} aria-hidden="true" />
                <div>
                  <strong>{panel.name}</strong>
                  <span className="panel-card-meta">
                    {panel.hidden ? "Oculto" : "Visible"}
                    {panel.locked ? " · Posición bloqueada" : ""}
                    {panel.lockContent ? " · Contenido bloqueado" : ""}
                    {" · "}
                    {Math.round(panel.width)} × {Math.round(panel.height)}
                    {" · "}
                    {monitorLabel(panel)}
                    {" · "}
                    {panel.items.length} {panel.items.length === 1 ? "acceso" : "accesos"}
                  </span>
                </div>
              </div>
              <div className="panel-card-actions">
                {panel.hidden ? (
                  <button
                    className="button button-secondary"
                    onClick={() => void run(() => panelApi.setHidden(panel.id, false))}
                  >
                    Mostrar
                  </button>
                ) : (
                  <button
                    className="button button-ghost"
                    onClick={() => void run(() => panelApi.setHidden(panel.id, true))}
                  >
                    Ocultar
                  </button>
                )}
                <button
                  className="button button-ghost"
                  onClick={() => void run(async () => {
                    const name = window.prompt("Nuevo nombre del panel", panel.name)?.trim();
                    if (name && name !== panel.name) {
                      await panelApi.update(panel.id, { name });
                    }
                  })}
                >
                  Renombrar
                </button>
                <button
                  className="button button-ghost"
                  onClick={() => void run(
                    () => panelApi.duplicate(panel.id),
                    `Panel “${panel.name}” duplicado. Sólo se copiaron las referencias.`,
                  )}
                >
                  Duplicar
                </button>
                <button
                  className="button button-ghost is-danger"
                  onClick={() => void run(async () => {
                    const confirmed = window.confirm(
                      `¿Eliminar el panel “${panel.name}”?\n\nSe borran sólo el panel y sus accesos. Ningún archivo real se toca.`,
                    );
                    if (confirmed) {
                      await panelApi.remove(panel.id);
                      onMessage(`Panel “${panel.name}” eliminado. No se tocó ningún archivo.`);
                    }
                  })}
                >
                  Eliminar
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
