import { useEffect, useState, type FormEvent } from "react";
import { drawerApi } from "../../services/drawerApi";
import type { Drawer } from "../../types/drawer";

interface DrawerCardProps {
  drawer: Drawer;
  onError: (message: string | null) => void;
}

const QUICK_COLORS = ["#293548", "#384E77", "#3F6655", "#704858", "#68553B"];

export function DrawerCard({ drawer, onError }: DrawerCardProps) {
  const [editingName, setEditingName] = useState(false);
  const [name, setName] = useState(drawer.name);
  const [busy, setBusy] = useState(false);

  useEffect(() => setName(drawer.name), [drawer.name]);

  async function run(action: () => Promise<unknown>) {
    setBusy(true);
    onError(null);
    try {
      await action();
    } catch (reason) {
      onError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  function submitName(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextName = name.trim();
    if (!nextName || nextName === drawer.name) {
      setName(drawer.name);
      setEditingName(false);
      return;
    }
    void run(async () => {
      await drawerApi.update(drawer.id, { name: nextName });
      setEditingName(false);
    });
  }

  function confirmDelete() {
    const confirmed = window.confirm(
      `¿Quitar el cajón visual “${drawer.name}”?\n\nNo se borrará su carpeta ni ningún archivo. Podrás recuperarlo desde:\n${drawer.folderPath}`,
    );
    if (confirmed) void run(() => drawerApi.remove(drawer.id));
  }

  return (
    <article className="drawer-card" style={{ "--drawer-accent": drawer.color } as React.CSSProperties}>
      <div className="drawer-card-accent" />
      <div className="drawer-card-main">
        <div className="drawer-card-title-row">
          {editingName ? (
            <form className="inline-name-form" onSubmit={submitName}>
              <input
                value={name}
                onChange={(event) => setName(event.target.value)}
                maxLength={80}
                autoFocus
                aria-label="Nuevo nombre"
              />
              <button className="icon-button" title="Guardar" aria-label="Guardar nombre">✓</button>
              <button
                className="icon-button"
                type="button"
                title="Cancelar"
                aria-label="Cancelar edición"
                onClick={() => {
                  setName(drawer.name);
                  setEditingName(false);
                }}
              >
                ×
              </button>
            </form>
          ) : (
            <>
              <h3>{drawer.name}</h3>
              <button
                className="icon-button subtle"
                onClick={() => setEditingName(true)}
                title="Renombrar"
                aria-label={`Renombrar ${drawer.name}`}
              >
                ✎
              </button>
            </>
          )}
        </div>
        <div className="drawer-meta">
          <span className={`status-pill ${drawer.hidden ? "is-hidden" : "is-visible"}`}>
            <span className="status-dot" />
            {drawer.hidden ? "Oculto" : "Visible"}
          </span>
          <span>{drawer.collapsed ? "Contraído" : "Expandido"}</span>
          <span>{drawer.locked ? "Posición bloqueada" : "Posición libre"}</span>
          <span>{drawer.items.length} {drawer.items.length === 1 ? "elemento" : "elementos"}</span>
          <span className="physical-path" title={drawer.folderPath}>Carpeta: {drawer.folderPath}</span>
        </div>
        <div className="drawer-quick-settings">
          <div className="quick-color-control">
            <span>Color</span>
            <div className="color-swatches" aria-label={`Colores rápidos para ${drawer.name}`}>
              {QUICK_COLORS.map((color) => (
                <button
                  key={color}
                  type="button"
                  className={drawer.color === color ? "is-selected" : ""}
                  style={{ backgroundColor: color }}
                  title={`Usar color ${color}`}
                  aria-label={`Usar color ${color}`}
                  disabled={busy}
                  onClick={() => void run(() => drawerApi.update(drawer.id, { color }))}
                />
              ))}
            </div>
            <input
              type="color"
              value={drawer.color}
              title="Elegir un color personalizado"
              aria-label="Elegir un color personalizado"
              disabled={busy}
              onChange={(event) => void run(() => drawerApi.update(drawer.id, { color: event.target.value }))}
            />
          </div>
          <label className="opacity-control">
            <span>Opacidad {Math.round(drawer.opacity * 100)}%</span>
            <input
              type="range"
              min="45"
              max="100"
              step="5"
              value={Math.round(drawer.opacity * 100)}
              disabled={busy}
              onChange={(event) =>
                void run(() => drawerApi.update(drawer.id, { opacity: Number(event.target.value) / 100 }))
              }
            />
          </label>
          <label className="switch-control">
            <input
              type="checkbox"
              checked={drawer.locked}
              disabled={busy}
              onChange={(event) =>
                void run(() => drawerApi.update(drawer.id, { locked: event.target.checked }))
              }
            />
            <span>Bloquear posición</span>
          </label>
        </div>
      </div>
      <div className="drawer-card-actions">
        <button
          className="button button-ghost"
          disabled={busy}
          onClick={() => void run(() => drawerApi.openFolder(drawer.id))}
        >
          Abrir carpeta
        </button>
        <button
          className="button button-secondary"
          disabled={busy}
          onClick={() => void run(() => drawerApi.setHidden(drawer.id, !drawer.hidden))}
        >
          {drawer.hidden ? "Mostrar" : "Ocultar"}
        </button>
        <button className="button button-danger" disabled={busy} onClick={confirmDelete}>
          Eliminar
        </button>
      </div>
    </article>
  );
}
