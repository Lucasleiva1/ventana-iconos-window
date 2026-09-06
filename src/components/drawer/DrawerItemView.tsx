import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import { createPortal } from "react-dom";
import {
  DRAWER_ITEM_NAMES_VISIBLE,
  DRAWER_STORAGE_BADGES_VISIBLE,
  SUBDRAWERS_ENABLED,
} from "../../features";
import { drawerApi } from "../../services/drawerApi";
import { iconService } from "../../services/iconService";
import { useDrawerDialogs } from "./DrawerDialogs";
import {
  hasInternalDrag,
  joinDrawerPath,
  parseInternalDrag,
} from "../../services/internalDrag";
import type {
  Drawer,
  DrawerItem,
  DrawerLevel,
  InternalDragPayload,
} from "../../types/drawer";

interface DrawerItemViewProps {
  drawer: Drawer;
  drawers: Drawer[];
  relativePath: string;
  item: DrawerItem;
  dragging?: boolean;
  dropTarget?: boolean;
  onNavigate: (relativePath: string) => void;
  onLevelChanged: (level?: DrawerLevel) => void;
  onReorder: (payload: InternalDragPayload) => Promise<void>;
  onMoveInto: (payload: InternalDragPayload, targetRelativePath: string) => Promise<void>;
  onFeedback: (message: string) => void;
}

const FALLBACK_ICONS: Record<DrawerItem["type"], string> = {
  folder: "▰",
  executable: "◆",
  shortcut: "↗",
  file: "▤",
};

export function DrawerItemView({
  drawer,
  drawers,
  relativePath,
  item,
  dragging,
  dropTarget,
  onNavigate,
  onLevelChanged,
  onReorder,
  onMoveInto,
  onFeedback,
}: DrawerItemViewProps) {
  const dialogs = useDrawerDialogs();
  const [icon, setIcon] = useState<string | null>(null);
  const [menuPosition, setMenuPosition] = useState<{ left: number; top: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const menuOpen = menuPosition !== null;

  useEffect(() => {
    let active = true;
    void iconService
      .getAtLevel(drawer.id, relativePath, item.id, item.iconKey)
      .then((value) => {
        if (active) setIcon(value);
      });
    return () => {
      active = false;
    };
  }, [drawer.id, item.iconKey, item.id, relativePath]);

  useEffect(() => {
    if (!menuOpen) return;
    const close = () => setMenuPosition(null);
    window.addEventListener("pointerdown", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("blur", close);
    };
  }, [menuOpen]);

  useLayoutEffect(() => {
    if (!menuPosition || !menuRef.current) return;
    const margin = 8;
    const bounds = menuRef.current.getBoundingClientRect();
    const left = Math.max(margin, Math.min(menuPosition.left, window.innerWidth - bounds.width - margin));
    const top = Math.max(margin, Math.min(menuPosition.top, window.innerHeight - bounds.height - margin));
    if (left !== menuPosition.left || top !== menuPosition.top) {
      setMenuPosition({ left, top });
    }
  }, [menuPosition]);

  async function openItem() {
    setMenuPosition(null);
    if (item.isSubdrawer) {
      onNavigate(joinDrawerPath(relativePath, item.physicalName));
      return;
    }
    try {
      await drawerApi.openLevelItem(drawer.id, relativePath, item.id);
    } catch (reason) {
      onFeedback(String(reason));
    }
  }

  async function run(event: MouseEvent<HTMLButtonElement>, action: () => Promise<unknown>) {
    event.stopPropagation();
    setMenuPosition(null);
    try {
      await action();
      onLevelChanged();
    } catch (reason) {
      onFeedback(String(reason));
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Enter") {
      event.preventDefault();
      void openItem();
    }
  }

  function beginNativeDrag(event: DragEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();
    setMenuPosition(null);
    void drawerApi.beginNativeItemDrag(drawer.id, relativePath, item.id)
      .catch((reason) => onFeedback(String(reason)));
  }

  function handleDrop(event: DragEvent<HTMLDivElement>) {
    const payload = parseInternalDrag(event);
    if (!payload || payload.itemId === item.id) return;
    event.preventDefault();
    event.stopPropagation();
    const sameLevel = payload.sourceDrawerId === drawer.id
      && payload.sourceRelativePath.toLowerCase() === relativePath.toLowerCase();
    const action = item.isSubdrawer && !sameLevel
      ? onMoveInto(payload, joinDrawerPath(relativePath, item.physicalName))
      : onReorder(payload);
    void action.catch((reason) => onFeedback(String(reason)));
  }

  const otherDrawers = drawers.filter((candidate) => candidate.id !== drawer.id);

  return (
    <div
      className={`drawer-item ${item.available ? "" : "is-unavailable"} ${item.isSubdrawer ? "is-subdrawer" : ""} ${dragging ? "is-reordering" : ""} ${dropTarget ? "is-order-target" : ""}`}
      data-item-id={item.id}
      role="button"
      tabIndex={0}
      draggable={false}
      title={`${item.displayName}\n${item.path}`}
      aria-label={`${item.displayName}${item.isSubdrawer ? ", subcajón" : ""}${item.available ? "" : ", no disponible"}`}
      onDragStart={(event) => event.preventDefault()}
      onDragOver={(event) => {
        if (hasInternalDrag(event)) {
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
        }
      }}
      onDrop={handleDrop}
      onDoubleClick={() => void openItem()}
      onKeyDown={handleKeyDown}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        setMenuPosition({ left: event.clientX, top: event.clientY });
      }}
    >
      <span className="drawer-item-icon" aria-hidden="true">
        {icon ? (
          <img src={icon} alt="" draggable={false} />
        ) : (
          <span className={`fallback-item-icon type-${item.type}`}>
            {item.isSubdrawer ? "▣" : FALLBACK_ICONS[item.type]}
          </span>
        )}
      </span>
      {DRAWER_ITEM_NAMES_VISIBLE && <span className="drawer-item-name">{item.displayName}</span>}
      {DRAWER_STORAGE_BADGES_VISIBLE && <span className={`storage-mode-badge is-${item.storageMode}`}>
        {item.isSubdrawer ? "Subcajón" : item.storageMode === "managed" ? "Guardado" : "Vínculo"}
      </span>}
      {item.storageMode === "managed" && (
        <button
          className="drawer-item-drag-out"
          type="button"
          draggable
          title="Arrastrar fuera del cajón. Clic para restaurar al Escritorio."
          aria-label={`Sacar ${item.displayName} del cajón`}
          onPointerDown={(event) => event.stopPropagation()}
          onDragStart={beginNativeDrag}
          onClick={(event) => void run(event, async () => {
            const message = await drawerApi.restoreLevelItem(drawer.id, relativePath, item.id);
            onFeedback(message);
          })}
        >
          ↗
        </button>
      )}
      {!item.available && <span className="unavailable-mark">No disponible</span>}
      {menuPosition && createPortal((
        <div
          ref={menuRef}
          className="item-context-menu"
          role="menu"
          style={{ left: menuPosition.left, top: menuPosition.top }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          <button type="button" role="menuitem" onClick={(event) => void run(event, () => drawerApi.openLevelItemLocation(drawer.id, relativePath, item.id))}>
            <strong>Abrir ubicación</strong>
            <span>{item.path}</span>
          </button>
          <button type="button" role="menuitem" onClick={(event) => {
            event.stopPropagation();
            setMenuPosition(null);
            void navigator.clipboard.writeText(item.path)
              .then(() => onFeedback("Ruta copiada."))
              .catch(() => onFeedback("No se pudo copiar la ruta."));
          }}>
            <strong>Copiar ruta</strong>
            <span>Ruta física completa</span>
          </button>
          {item.storageMode === "managed" ? (
            <>
              <button type="button" role="menuitem" onClick={(event) => void run(event, async () => {
                const message = await drawerApi.restoreLevelItem(drawer.id, relativePath, item.id);
                onFeedback(message);
              })}>
                <strong>Restaurar al Escritorio</strong>
                <span>Lo devuelve al Escritorio real</span>
              </button>
              {SUBDRAWERS_ENABLED && item.type === "folder" && !item.isSubdrawer && (
                <button type="button" role="menuitem" onClick={(event) => void run(event, () => drawerApi.convertFolderToSubdrawer(drawer.id, relativePath, item.id))}>
                  <strong>Convertir en subcajón</strong>
                  <span>Conserva todo el contenido</span>
                </button>
              )}
              {SUBDRAWERS_ENABLED && item.isSubdrawer && (
                <>
                  <button type="button" role="menuitem" onClick={(event) => void run(event, async () => {
                    const name = await dialogs.askText({
                      title: "Renombrar subcajón",
                      label: "Nuevo nombre",
                      defaultValue: item.displayName,
                      confirmLabel: "Renombrar",
                    });
                    if (name && name !== item.displayName) {
                      await drawerApi.renameSubdrawer(drawer.id, relativePath, item.id, name);
                    }
                  })}>
                    <strong>Renombrar subcajón</strong>
                    <span>Renombra su carpeta física</span>
                  </button>
                  <button type="button" role="menuitem" onClick={(event) => void run(event, () => drawerApi.convertSubdrawerToFolder(drawer.id, relativePath, item.id))}>
                    <strong>Convertir en carpeta normal</strong>
                    <span>No borra su contenido</span>
                  </button>
                </>
              )}
            </>
          ) : (
            <button className="link-removal" type="button" role="menuitem" onClick={(event) => void run(event, () => drawerApi.removeLevelLink(drawer.id, relativePath, item.id))}>
              <strong>Quitar vínculo</strong>
              <span>No modifica el archivo original</span>
            </button>
          )}
          {otherDrawers.map((target) => (
            <button key={target.id} type="button" role="menuitem" onClick={(event) => void run(event, () => drawerApi.moveItemBetweenLevels(drawer.id, relativePath, item.id, target.id, ""))}>
              <strong>Mover a {target.name}</strong>
              <span>{item.storageMode === "linked" ? "Mueve sólo la referencia" : "Mueve el elemento físico"}</span>
            </button>
          ))}
        </div>
      ), document.body)}
    </div>
  );
}
