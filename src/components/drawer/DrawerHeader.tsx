import type { PointerEvent } from "react";
import { SUBDRAWERS_ENABLED } from "../../features";
import { drawerApi } from "../../services/drawerApi";
import type { Drawer, DrawerLevel } from "../../types/drawer";

interface DrawerHeaderProps {
  drawer: Drawer;
  itemCount?: number;
  /** Nivel abierto: dentro de un subcajón la cabecera muestra ese nombre. */
  level?: DrawerLevel;
  onNavigate?: (relativePath: string) => void;
  onCreateSubdrawer?: () => void;
  onRefresh?: () => void;
  settingsOpen: boolean;
  onToggleSettings: () => void;
}

/**
 * Único lugar de mando del Cajón: nombre del nivel abierto, cantidad de
 * elementos y todas las acciones. Antes había además una barra dentro del
 * cuerpo que repetía el nombre; las acciones viven acá y el cuerpo queda sólo
 * para el contenido.
 */
export function DrawerHeader({
  drawer,
  itemCount,
  level,
  onNavigate,
  onCreateSubdrawer,
  onRefresh,
  settingsOpen,
  onToggleSettings,
}: DrawerHeaderProps) {
  function beginDrag(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0 || drawer.locked) return;
    if ((event.target as HTMLElement).closest("button")) return;
    void drawerApi.beginDrag();
  }

  const insideSubdrawer = Boolean(level?.relativePath);
  const currentName = insideSubdrawer
    ? level?.breadcrumbs.at(-1)?.name ?? drawer.name
    : drawer.name;
  // Con la ruta completa en el tooltip no hace falta una barra aparte para
  // saber dónde se está parado.
  const fullPath = level?.breadcrumbs.map((crumb) => crumb.name).join(" / ") ?? drawer.name;

  return (
    <header
      className={`drawer-header ${drawer.locked ? "is-locked" : ""}`}
      onPointerDown={beginDrag}
    >
      <div className="drawer-title-wrap">
        {insideSubdrawer ? (
          <button
            type="button"
            className="header-button back-button"
            title={`Volver a ${level?.breadcrumbs.at(-2)?.name ?? drawer.name}`}
            aria-label="Volver al nivel anterior"
            onClick={() => onNavigate?.(level?.breadcrumbs.at(-2)?.relativePath ?? "")}
          >
            ‹
          </button>
        ) : (
          <span className="drawer-title-mark" aria-hidden="true" />
        )}
        <h1 title={fullPath}>{currentName}</h1>
        <span className="drawer-item-count" title={`${itemCount ?? drawer.items.length} elementos`}>
          {itemCount ?? drawer.items.length}
        </span>
        {drawer.locked && <span className="lock-indicator" title="Posición bloqueada">▣</span>}
      </div>
      <div className="drawer-header-actions">
        {SUBDRAWERS_ENABLED && !drawer.collapsed && onCreateSubdrawer && (
          <button
            className="header-button"
            title="Nuevo subcajón"
            aria-label="Crear un subcajón"
            onClick={onCreateSubdrawer}
          >
            +▣
          </button>
        )}
        {!drawer.collapsed && onRefresh && (
          <button
            className="header-button"
            title="Actualizar"
            aria-label="Actualizar el contenido"
            onClick={onRefresh}
          >
            ↻
          </button>
        )}
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
