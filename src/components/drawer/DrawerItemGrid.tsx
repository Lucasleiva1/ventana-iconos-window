import type { DragEvent } from "react";
import { drawerApi } from "../../services/drawerApi";
import { INTERNAL_ITEM_MIME, parseInternalDrag } from "../../services/internalDrag";
import type {
  Drawer,
  DrawerItem,
  DrawerLevel,
  InternalDragPayload,
} from "../../types/drawer";
import { DrawerItemView } from "./DrawerItemView";

interface DrawerItemGridProps {
  drawer: Drawer;
  drawers: Drawer[];
  level: DrawerLevel;
  onNavigate: (relativePath: string) => void;
  onLevelChanged: (level?: DrawerLevel) => void;
  onFeedback: (message: string) => void;
}

export function DrawerItemGrid({
  drawer,
  drawers,
  level,
  onNavigate,
  onLevelChanged,
  onFeedback,
}: DrawerItemGridProps) {
  const items = [...level.items].sort((a, b) => a.order - b.order);

  async function movePayload(payload: InternalDragPayload, targetRelativePath: string) {
    if (
      payload.sourceDrawerId === drawer.id
      && payload.sourceRelativePath.toLowerCase() === targetRelativePath.toLowerCase()
    ) {
      return;
    }
    await drawerApi.moveItemBetweenLevels(
      payload.sourceDrawerId,
      payload.sourceRelativePath,
      payload.itemId,
      drawer.id,
      targetRelativePath,
    );
    onLevelChanged();
    onFeedback("Elemento movido sin sobrescribir archivos existentes.");
  }

  async function reorder(payload: InternalDragPayload, target: DrawerItem) {
    if (
      payload.sourceDrawerId !== drawer.id
      || payload.sourceRelativePath.toLowerCase() !== level.relativePath.toLowerCase()
    ) {
      await movePayload(payload, level.relativePath);
      return;
    }
    const ids = items.map((item) => item.id);
    const sourceIndex = ids.indexOf(payload.itemId);
    const targetIndex = ids.indexOf(target.id);
    if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) return;
    const [moved] = ids.splice(sourceIndex, 1);
    ids.splice(targetIndex, 0, moved);
    const updated = await drawerApi.reorderLevel(drawer.id, level.relativePath, ids);
    onLevelChanged(updated);
  }

  function handleGridDrop(event: DragEvent<HTMLDivElement>) {
    const payload = parseInternalDrag(event);
    if (!payload) return;
    event.preventDefault();
    void movePayload(payload, level.relativePath).catch((reason) => onFeedback(String(reason)));
  }

  return (
    <div
      className={`drawer-item-grid icon-size-${drawer.iconSize}`}
      onDragOver={(event) => {
        if (event.dataTransfer.types.includes(INTERNAL_ITEM_MIME)) {
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
        }
      }}
      onDrop={handleGridDrop}
    >
      {items.map((item) => (
        <DrawerItemView
          key={item.id}
          drawer={drawer}
          drawers={drawers}
          relativePath={level.relativePath}
          item={item}
          onNavigate={onNavigate}
          onLevelChanged={onLevelChanged}
          onReorder={(payload) => reorder(payload, item)}
          onMoveInto={movePayload}
          onFeedback={onFeedback}
        />
      ))}
    </div>
  );
}
