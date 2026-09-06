import { memo, useEffect, useState } from "react";
import { panelApi } from "../../services/panelApi";
import type { PanelItem } from "../../types/panel";
import type { LabelMode } from "./usePanelLayout";

const FALLBACK_ICONS: Record<PanelItem["type"], string> = {
  folder: "▰",
  executable: "◆",
  shortcut: "↗",
  file: "▤",
};

interface PanelItemViewProps {
  panelId: string;
  item: PanelItem;
  iconSize: number;
  labelMode: LabelMode;
  dragging: boolean;
  dropTarget: boolean;
  selected: boolean;
  onOpen: (item: PanelItem) => void;
  onContextMenu: (item: PanelItem, x: number, y: number) => void;
}

/** Nombre corto para cuando sólo entra una línea de texto. */
function shortName(name: string) {
  const trimmed = name.trim();
  const firstWord = trimmed.split(/\s+/)[0];
  return firstWord.length >= 3 ? firstWord : trimmed;
}

function PanelItemViewBase({
  panelId,
  item,
  iconSize,
  labelMode,
  dragging,
  dropTarget,
  selected,
  onOpen,
  onContextMenu,
}: PanelItemViewProps) {
  const [icon, setIcon] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void panelApi.getItemIcon(panelId, item.id, item.iconKey).then((value) => {
      if (active) setIcon(value);
    });
    return () => {
      active = false;
    };
  }, [panelId, item.id, item.iconKey]);

  const isDrawer = item.drawerId !== null;
  const tooltip = item.available
    ? `${item.displayName}\n${item.path}`
    : `${item.displayName}\nNo disponible · ${item.path}`;

  return (
    <div
      className={[
        "panel-item",
        item.available ? "" : "is-unavailable",
        isDrawer ? "is-drawer" : "",
        dragging ? "is-dragging" : "",
        dropTarget ? "is-drop-target" : "",
        selected ? "is-selected" : "",
      ].filter(Boolean).join(" ")}
      data-panel-item={item.id}
      role="button"
      tabIndex={0}
      draggable={false}
      title={tooltip}
      aria-label={`${item.displayName}${isDrawer ? ", cajón" : ""}${item.available ? "" : ", no disponible"}`}
      aria-selected={selected}
      onDragStart={(event) => event.preventDefault()}
      onDoubleClick={() => onOpen(item)}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          onOpen(item);
        }
      }}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onContextMenu(item, event.clientX, event.clientY);
      }}
    >
      <span
        className="panel-item-icon"
        style={{ width: iconSize, height: iconSize }}
        aria-hidden="true"
      >
        {icon ? (
          <img src={icon} alt="" draggable={false} />
        ) : (
          <span
            className={`panel-fallback-icon type-${item.type}`}
            style={{ fontSize: Math.round(iconSize * 0.6) }}
          >
            {isDrawer ? "▣" : FALLBACK_ICONS[item.type]}
          </span>
        )}
      </span>
      {labelMode !== "none" && (
        <span className={`panel-item-name is-${labelMode}`}>
          {labelMode === "full" ? item.displayName : shortName(item.displayName)}
        </span>
      )}
      {!item.available && <span className="panel-item-broken" aria-hidden="true">!</span>}
    </div>
  );
}

/**
 * Memoizado a propósito: al redimensionar el Panel sólo cambian `iconSize` y
 * `labelMode`, así que los elementos que no cambian no se vuelven a dibujar.
 */
export const PanelItemView = memo(PanelItemViewBase);
