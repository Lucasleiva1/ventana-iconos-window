import type { Drawer } from "../../types/drawer";
import type { Panel, PanelPatch } from "../../types/panel";
import { MAX_ICON, MIN_ICON } from "./usePanelLayout";

interface PanelOptionsMenuProps {
  panel: Panel;
  drawers: Drawer[];
  onPatch: (patch: PanelPatch) => void;
  onRename: () => void;
  onDuplicate: () => void;
  onAddDrawer: (drawerId: string) => void;
  onToggleExpanded: () => void;
  onRefresh: () => void;
  onHide: () => void;
  onDelete: () => void;
  onClose: () => void;
}

const DENSITIES = [
  { value: "compact", label: "Compacta" },
  { value: "normal", label: "Normal" },
  { value: "wide", label: "Amplia" },
] as const;

const BACKGROUNDS = [
  { value: "solid", label: "Sólido" },
  { value: "translucent", label: "Transparente" },
  { value: "glass", label: "Cristal" },
  { value: "minimal", label: "Mínimo" },
] as const;

export function PanelOptionsMenu({
  panel,
  drawers,
  onPatch,
  onRename,
  onDuplicate,
  onAddDrawer,
  onToggleExpanded,
  onRefresh,
  onHide,
  onDelete,
  onClose,
}: PanelOptionsMenuProps) {
  const available = drawers.filter(
    (drawer) => !panel.items.some((item) => item.drawerId === drawer.id),
  );

  return (
    <div
      className="panel-options"
      role="menu"
      aria-label={`Opciones de ${panel.name}`}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <div className="panel-options-actions">
        <button type="button" onClick={onRename}>Renombrar</button>
        <button type="button" onClick={onDuplicate}>Duplicar</button>
        <button type="button" onClick={onRefresh}>Actualizar</button>
      </div>

      <label className="panel-option-row">
        <span>Tamaño de iconos</span>
        <select
          value={panel.iconMode}
          onChange={(event) => onPatch({ iconMode: event.target.value as Panel["iconMode"] })}
        >
          <option value="auto">Automático</option>
          <option value="manual">Manual</option>
        </select>
      </label>

      {panel.iconMode === "manual" && (
        <label className="panel-option-row">
          <span>{Math.round(panel.manualIconSize)} px</span>
          <input
            type="range"
            min={MIN_ICON}
            max={MAX_ICON}
            step={4}
            value={Math.round(panel.manualIconSize)}
            onChange={(event) => onPatch({ manualIconSize: Number(event.target.value) })}
          />
        </label>
      )}

      <label className="panel-option-row">
        <span>Densidad</span>
        <select
          value={panel.density}
          onChange={(event) => onPatch({ density: event.target.value as Panel["density"] })}
        >
          {DENSITIES.map((option) => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>
      </label>

      <label className="panel-option-row">
        <span>Fondo</span>
        <select
          value={panel.backgroundStyle}
          onChange={(event) => onPatch({
            backgroundStyle: event.target.value as Panel["backgroundStyle"],
          })}
        >
          {BACKGROUNDS.map((option) => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>
      </label>

      <label className="panel-option-row">
        <span>Color</span>
        <input
          type="color"
          value={panel.color}
          onChange={(event) => onPatch({ color: event.target.value })}
        />
      </label>

      <label className="panel-option-row">
        <span>Opacidad · {Math.round(panel.opacity * 100)} %</span>
        <input
          type="range"
          min={0}
          max={100}
          value={Math.round(panel.opacity * 100)}
          onChange={(event) => onPatch({ opacity: Number(event.target.value) / 100 })}
        />
      </label>

      <label className="panel-option-row">
        <span>Cabecera</span>
        <select
          value={panel.headerMode}
          onChange={(event) => onPatch({ headerMode: event.target.value as Panel["headerMode"] })}
        >
          <option value="normal">Normal</option>
          <option value="compact">Compacta</option>
        </select>
      </label>

      <label className="panel-option-row is-switch">
        <span>Mostrar título</span>
        <input
          type="checkbox"
          checked={panel.showTitle}
          onChange={(event) => onPatch({ showTitle: event.target.checked })}
        />
      </label>

      <label className="panel-option-row is-switch">
        <span>Bloquear posición</span>
        <input
          type="checkbox"
          checked={panel.locked}
          onChange={(event) => onPatch({ locked: event.target.checked })}
        />
      </label>

      <label className="panel-option-row is-switch">
        <span>Bloquear contenido</span>
        <input
          type="checkbox"
          checked={panel.lockContent}
          onChange={(event) => onPatch({ lockContent: event.target.checked })}
        />
      </label>

      <label className="panel-option-row is-switch">
        <span>Ajustar a bordes</span>
        <input
          type="checkbox"
          checked={panel.snapEnabled}
          onChange={(event) => onPatch({ snapEnabled: event.target.checked })}
        />
      </label>

      {available.length > 0 && (
        <div className="panel-option-row is-stack">
          <span>Agregar un cajón</span>
          <div className="panel-drawer-chips">
            {available.map((drawer) => (
              <button key={drawer.id} type="button" onClick={() => onAddDrawer(drawer.id)}>
                ▣ {drawer.name}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="panel-options-actions">
        <button type="button" onClick={onToggleExpanded} disabled={panel.locked}>
          {panel.expanded ? "Restaurar tamaño" : "Expandir al escritorio"}
        </button>
        <button type="button" onClick={onHide}>Ocultar</button>
        <button type="button" className="is-danger" onClick={onDelete}>Eliminar panel</button>
      </div>

      <button type="button" className="panel-options-close" onClick={onClose}>Cerrar</button>
    </div>
  );
}
