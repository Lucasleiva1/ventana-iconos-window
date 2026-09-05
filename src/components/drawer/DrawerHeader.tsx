import type { PointerEvent } from "react";
import { drawerApi } from "../../services/drawerApi";
import type { Drawer } from "../../types/drawer";

interface DrawerHeaderProps {
  drawer: Drawer;
  itemCount?: number;
  settingsOpen: boolean;
  onToggleSettings: () => void;
}

export function DrawerHeader({ drawer, itemCount, settingsOpen, onToggleSettings }: DrawerHeaderProps) {
  function beginDrag(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0 || drawer.locked) return;
    if ((event.target as HTMLElement).closest("button")) return;
    void drawerApi.beginDrag();
  }

  return (
    <header
      className={`drawer-header ${drawer.locked ? "is-locked" : ""}`}
      onPointerDown={beginDrag}
    >
      <div className="drawer-title-wrap">
        <span className="drawer-title-mark" aria-hidden="true" />
        <h1>{drawer.name}</h1>
        <span className="drawer-item-count" title={`${itemCount ?? drawer.items.length} elementos`}>
          {itemCount ?? drawer.items.length}
        </span>
        {drawer.locked && <span className="lock-indicator" title="Posición bloqueada">▣</span>}
      </div>
      <div className="drawer-header-actions">
        <button
          className="header-button collapse-button"
          title={drawer.collapsed ? "Expandir" : "Contraer"}
          aria-label={drawer.collapsed ? "Expandir cajón" : "Contraer cajón"}
          onClick={() => void drawerApi.setCollapsed(drawer.id, !drawer.collapsed)}
        >
          {drawer.collapsed ? "+" : "−"}
        </button>
        <button
          className={`header-button menu-button ${settingsOpen ? "is-active" : ""}`}
          title="Opciones"
          aria-label="Abrir opciones del cajón"
          aria-expanded={settingsOpen}
          onClick={onToggleSettings}
        >
          •••
        </button>
      </div>
    </header>
  );
}
