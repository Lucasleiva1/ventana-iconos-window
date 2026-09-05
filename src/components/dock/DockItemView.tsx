import { useEffect, useState } from "react";
import { dockApi } from "../../services/dockApi";
import type { DockItem } from "../../types/dock";

const FALLBACK_ICONS: Record<DockItem["type"], string> = {
  folder: "▰",
  executable: "◆",
  shortcut: "↗",
  file: "▤",
};

interface DockItemViewProps {
  item: DockItem;
  dragging: boolean;
  dropTarget: boolean;
  onContextMenu: (item: DockItem) => void;
}

export function DockItemView({ item, dragging, dropTarget, onContextMenu }: DockItemViewProps) {
  const [icon, setIcon] = useState<string | null>(null);

  useEffect(() => {
    if (item.kind === "separator") {
      setIcon(null);
      return;
    }
    let active = true;
    void dockApi.getItemIcon(item.id, item.iconKey).then((value) => {
      if (active) setIcon(value);
    });
    return () => {
      active = false;
    };
  }, [item.id, item.iconKey]);

  if (item.kind === "separator") {
    return (
      <div
        className={`dock-separator ${dragging ? "is-dragging" : ""} ${dropTarget ? "is-drop-target" : ""}`}
        data-dock-item={item.id}
        role="separator"
        tabIndex={0}
        title="Separador · clic derecho para quitar"
        aria-label="Separador del Dock"
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onContextMenu(item);
        }}
      >
        <span aria-hidden="true" />
      </div>
    );
  }

  return (
    <div
      className={`dock-item ${item.available ? "" : "is-unavailable"} ${dragging ? "is-dragging" : ""} ${dropTarget ? "is-drop-target" : ""}`}
      data-dock-item={item.id}
      role="button"
      tabIndex={0}
      draggable={false}
      title={item.available ? `${item.displayName}\n${item.path}` : `${item.displayName}\nNo disponible · ${item.path}`}
      aria-label={`${item.displayName}${item.available ? "" : ", no disponible"}`}
      onDragStart={(event) => event.preventDefault()}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onContextMenu(item);
      }}
    >
      <span className="dock-item-icon" aria-hidden="true">
        {icon ? (
          <img src={icon} alt="" draggable={false} />
        ) : (
          <span className={`dock-fallback-icon type-${item.type}`}>{FALLBACK_ICONS[item.type]}</span>
        )}
      </span>
      {!item.available && <span className="dock-item-broken" aria-hidden="true">!</span>}
    </div>
  );
}
