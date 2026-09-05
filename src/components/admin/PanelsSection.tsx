import { useState, type FormEvent } from "react";
import { panelApi } from "../../services/panelApi";
import type { MonitorInfo } from "../../types/drawer";
import type { Panel } from "../../types/panel";

interface PanelsSectionProps {
  panels: Panel[];
  monitors: MonitorInfo[];
  onError: (message: string | null) => void;
  onMessage: (message: string) => void;
}

export function PanelsSection({ panels, monitors, onError, onMessage }: PanelsSectionProps) {
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
                    {panel.locked ? " · Bloqueado" : ""}
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
