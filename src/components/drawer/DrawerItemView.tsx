import {
  useEffect,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import { drawerApi } from "../../services/drawerApi";
import { iconService } from "../../services/iconService";
import {
  INTERNAL_ITEM_MIME,
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
  onNavigate,
  onLevelChanged,
  onReorder,
  onMoveInto,
  onFeedback,
}: DrawerItemViewProps) {
  const [icon, setIcon] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);

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
    const close = () => setMenuOpen(false);
    window.addEventListener("pointerdown", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("blur", close);
    };
  }, [menuOpen]);

  async function openItem() {
    setMenuOpen(false);
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
    setMenuOpen(false);
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

  function beginInternalDrag(event: DragEvent<HTMLDivElement>) {
    const payload: InternalDragPayload = {
      sourceDrawerId: drawer.id,
      sourceRelativePath: relativePath,
      itemId: item.id,
    };
    const serialized = JSON.stringify(payload);
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData(INTERNAL_ITEM_MIME, serialized);
    event.dataTransfer.setData("text/plain", serialized);
  }

  function handleDrop(event: DragEvent<HTMLDivElement>) {
    const payload = parseInternalDrag(event);
    if (!payload || payload.itemId === item.id) return;
    event.preventDefault();
    event.stopPropagation();
    const action = item.isSubdrawer
      ? onMoveInto(payload, joinDrawerPath(relativePath, item.physicalName))
      : onReorder(payload);
    void action.catch((reason) => onFeedback(String(reason)));
  }

  const otherDrawers = drawers.filter((candidate) => candidate.id !== drawer.id);

  return (
    <div
      className={`drawer-item ${item.available ? "" : "is-unavailable"} ${item.isSubdrawer ? "is-subdrawer" : ""}`}
      role="button"
      tabIndex={0}
      draggable
      title={`${item.displayName}\n${item.path}`}
      aria-label={`${item.displayName}${item.isSubdrawer ? ", subcajón" : ""}${item.available ? "" : ", no disponible"}`}
      onDragStart={beginInternalDrag}
      onDragOver={(event) => {
        if (event.dataTransfer.types.includes(INTERNAL_ITEM_MIME)) {
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
        }
      }}
      onDrop={handleDrop}
      onDoubleClick={() => void openItem()}
      onKeyDown={handleKeyDown}
      onContextMenu={(event) => {
        event.preventDefault();
        setMenuOpen(true);
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
      <span className="drawer-item-name">{item.displayName}</span>
      <span className={`storage-mode-badge is-${item.storageMode}`}>
        {item.isSubdrawer ? "Subcajón" : item.storageMode === "managed" ? "Guardado" : "Vínculo"}
      </span>
      {!item.available && <span className="unavailable-mark">No disponible</span>}
      {menuOpen && (
        <div
          className="item-context-menu"
          role="menu"
          onPointerDown={(event) => event.stopPropagation()}
        >
          <button type="button" role="menuitem" onClick={(event) => void run(event, () => drawerApi.openLevelItemLocation(drawer.id, relativePath, item.id))}>
            <strong>Abrir ubicación</strong>
            <span>{item.path}</span>
          </button>
          <button type="button" role="menuitem" onClick={(event) => {
            event.stopPropagation();
            setMenuOpen(false);
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
              {item.type === "folder" && !item.isSubdrawer && (
                <button type="button" role="menuitem" onClick={(event) => void run(event, () => drawerApi.convertFolderToSubdrawer(drawer.id, relativePath, item.id))}>
                  <strong>Convertir en subcajón</strong>
                  <span>Conserva todo el contenido</span>
                </button>
              )}
              {item.isSubdrawer && (
                <>
                  <button type="button" role="menuitem" onClick={(event) => void run(event, async () => {
                    const name = window.prompt("Nuevo nombre del subcajón", item.displayName)?.trim();
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
      )}
    </div>
  );
}
