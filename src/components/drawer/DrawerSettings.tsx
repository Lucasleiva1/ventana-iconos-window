import { useEffect, useRef, useState, type FormEvent } from "react";
import { drawerApi } from "../../services/drawerApi";
import type { Drawer } from "../../types/drawer";
import { useDrawerDialogs } from "./DrawerDialogs";

interface DrawerSettingsProps {
  drawer: Drawer;
  onClose: () => void;
}

export function DrawerSettings({ drawer, onClose }: DrawerSettingsProps) {
  const dialogs = useDrawerDialogs();
  const [name, setName] = useState(drawer.name);
  const [error, setError] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => setName(drawer.name), [drawer.name]);

  useEffect(() => {
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  async function run(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
    } catch (reason) {
      setError(String(reason));
    }
  }

  function rename(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextName = name.trim();
    if (nextName && nextName !== drawer.name) {
      void run(() => drawerApi.update(drawer.id, { name: nextName }));
    }
  }

  async function remove() {
    const confirmed = await dialogs.confirm({
      title: `¿Quitar el cajón “${drawer.name}”?`,
      message:
        `La carpeta y todos sus archivos quedan seguros en ${drawer.folderPath}. `
        + "Sólo se quita la ventana del Escritorio.",
      confirmLabel: "Quitar cajón",
      danger: true,
    });
    if (confirmed) void run(() => drawerApi.remove(drawer.id));
  }

  return (
    <div className="settings-backdrop" onPointerDown={onClose}>
      <div
        ref={menuRef}
        className="drawer-settings"
        role="dialog"
        aria-label={`Opciones de ${drawer.name}`}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <div className="settings-heading">
          <strong>Opciones</strong>
          <button className="icon-button" onClick={onClose} aria-label="Cerrar opciones">×</button>
        </div>
        <form className="settings-name" onSubmit={rename}>
          <label htmlFor="settings-name-input">Nombre</label>
          <div>
            <input
              id="settings-name-input"
              value={name}
              maxLength={80}
              onChange={(event) => setName(event.target.value)}
            />
            <button className="small-action" aria-label="Guardar nombre">Guardar</button>
          </div>
        </form>
        <label className="settings-field color-field">
          <span>Color</span>
          <input
            type="color"
            value={drawer.color}
            onChange={(event) => void run(() => drawerApi.update(drawer.id, { color: event.target.value }))}
          />
        </label>
        <label className="settings-field opacity-field">
          <span>Opacidad</span>
          <strong>{Math.round(drawer.opacity * 100)}%</strong>
          <input
            type="range"
            min="0"
            max="100"
            step="5"
            value={Math.round(drawer.opacity * 100)}
            onChange={(event) =>
              void run(() => drawerApi.update(drawer.id, { opacity: Number(event.target.value) / 100 }))
            }
          />
        </label>
        <div className="settings-field icon-size-field">
          <span>Tamaño de iconos</span>
          <div className="icon-size-options" role="group" aria-label="Tamaño de iconos">
            {(["small", "medium", "large"] as const).map((size) => (
              <button
                key={size}
                type="button"
                className={drawer.iconSize === size ? "is-selected" : ""}
                aria-pressed={drawer.iconSize === size}
                onClick={() => void run(() => drawerApi.setIconSize(drawer.id, size))}
              >
                {size === "small" ? "Chico" : size === "medium" ? "Medio" : "Grande"}
              </button>
            ))}
          </div>
        </div>
        <div className="settings-divider" />
        <div className="drawer-location">
          <span>Ubicación física</span>
          <code title={drawer.folderPath}>{drawer.folderPath}</code>
          <button
            type="button"
            className="settings-action"
            onClick={() => void run(() => drawerApi.openFolder(drawer.id))}
          >
            <span>↗</span>
            Abrir ubicación
          </button>
          <button
            type="button"
            className="settings-action"
            onClick={() => void navigator.clipboard.writeText(drawer.folderPath)
              .catch(() => setError("No se pudo copiar la ruta."))}
          >
            <span>⧉</span>
            Copiar ruta
          </button>
        </div>
        <div className="settings-divider" />
        <button
          className="settings-action"
          onClick={() => void run(() => drawerApi.update(drawer.id, { locked: !drawer.locked }))}
        >
          <span>{drawer.locked ? "□" : "▣"}</span>
          {drawer.locked ? "Desbloquear posición" : "Bloquear posición"}
        </button>
        <button
          className="settings-action"
          onClick={() => void run(() => drawerApi.setCollapsed(drawer.id, !drawer.collapsed))}
        >
          <span>{drawer.collapsed ? "+" : "−"}</span>
          {drawer.collapsed ? "Expandir" : "Contraer"}
        </button>
        <button
          className="settings-action"
          onClick={() => void run(() => drawerApi.setHidden(drawer.id, true))}
        >
          <span>◌</span>
          Ocultar
        </button>
        <div className="settings-divider" />
        <button className="settings-action danger" onClick={remove}>
          <span>×</span>
          Eliminar cajón
        </button>
        {error && <p className="settings-error">{error}</p>}
      </div>
    </div>
  );
}
