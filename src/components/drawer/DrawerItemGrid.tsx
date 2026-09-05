import { useLayoutEffect, useRef, useState, type PointerEvent, type DragEvent } from "react";
import { drawerApi } from "../../services/drawerApi";
import { hasInternalDrag, parseInternalDrag } from "../../services/internalDrag";
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
  const storedItems = [...level.items].sort((a, b) => a.order - b.order);
  const [preview, setPreview] = useState<string[] | null>(null);
  const previewRef = useRef<string[] | null>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const previousRects = useRef(new Map<string, DOMRect>());
  const items = preview
    ? [...storedItems].sort((a, b) => preview.indexOf(a.id) - preview.indexOf(b.id))
    : storedItems;

  useLayoutEffect(() => {
    const next = new Map<string, DOMRect>();
    gridRef.current?.querySelectorAll<HTMLElement>("[data-item-id]").forEach((cell) => {
      const id = cell.dataset.itemId!;
      const old = previousRects.current.get(id);
      cell.getAnimations().forEach((animation) => animation.cancel());
      const rect = cell.getBoundingClientRect();
      next.set(id, rect);
      if (old && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        const x = old.left - rect.left;
        const y = old.top - rect.top;
        if (x || y) cell.animate([
          { transform: `translate(${x}px, ${y}px)` },
          { transform: "translate(0, 0)" },
        ], { duration: 180, easing: "ease-out" });
      }
    });
    previousRects.current = next;
  }, [items.map((item) => item.id).join("|")]);
  const gesture = useRef<{ id: string; pointer: number; x: number; y: number; active: boolean } | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const [targetId, setTargetId] = useState<string | null>(null);
  const saving = useRef(false);

  function pointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || saving.current) return;
    const target = event.target as HTMLElement;
    if (target.closest("button, [role=menu]")) return;
    const cell = target.closest<HTMLElement>("[data-item-id]");
    if (!cell) return;
    gesture.current = { id: cell.dataset.itemId!, pointer: event.pointerId, x: event.clientX, y: event.clientY, active: false };
    previewRef.current = storedItems.map((item) => item.id);
  }

  function destination(event: PointerEvent<HTMLDivElement>) {
    const bounds = event.currentTarget.getBoundingClientRect();
    if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom) return undefined;
    // Hit-test layout slots, not animated rectangles passing under the pointer.
    for (const [id, rect] of previousRects.current) {
      if (event.clientX >= rect.left && event.clientX <= rect.right && event.clientY >= rect.top && event.clientY <= rect.bottom) return id;
    }
    return undefined;
  }

  function pointerMove(event: PointerEvent<HTMLDivElement>) {
    const current = gesture.current;
    if (!current || current.pointer !== event.pointerId) return;
    if (!current.active && Math.hypot(event.clientX - current.x, event.clientY - current.y) < 6) return;
    if (!current.active) {
      current.active = true;
      event.currentTarget.setPointerCapture(event.pointerId);
    }
    setDragging(current.id);
    const target = destination(event);
    setTargetId(target ?? null);
    if (target !== undefined && target !== current.id) {
      const ids = [...(previewRef.current ?? storedItems.map((item) => item.id))];
      const from = ids.indexOf(current.id);
      const to = target === null ? ids.length - 1 : ids.indexOf(target);
      if (from >= 0 && to >= 0 && from !== to) {
        ids.splice(from, 1);
        ids.splice(to, 0, current.id);
        previewRef.current = ids;
        setPreview(ids);
      }
    }
    event.preventDefault();
  }

  function cancelPointer() {
    gesture.current = null;
    setDragging(null);
    setTargetId(null);
    previewRef.current = null;
    setPreview(null);
  }

  function pointerUp(event: PointerEvent<HTMLDivElement>) {
    const current = gesture.current;
    if (!current || current.pointer !== event.pointerId) return;
    const target = destination(event);
    const ids = previewRef.current;
    gesture.current = null;
    setDragging(null);
    setTargetId(null);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (!current.active || target === undefined || !ids) { cancelPointer(); return; }
    saving.current = true;
    void drawerApi.reorderLevel(drawer.id, level.relativePath, ids)
      .then(onLevelChanged)
      .catch((reason) => onFeedback(String(reason)))
      .finally(() => { saving.current = false; cancelPointer(); });
  }

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

  async function reorderToEnd(payload: InternalDragPayload) {
    if (
      payload.sourceDrawerId !== drawer.id
      || payload.sourceRelativePath.toLowerCase() !== level.relativePath.toLowerCase()
    ) {
      await movePayload(payload, level.relativePath);
      return;
    }
    const ids = items.map((item) => item.id);
    const sourceIndex = ids.indexOf(payload.itemId);
    if (sourceIndex < 0 || sourceIndex === ids.length - 1) return;
    const [moved] = ids.splice(sourceIndex, 1);
    ids.push(moved);
    const updated = await drawerApi.reorderLevel(drawer.id, level.relativePath, ids);
    onLevelChanged(updated);
  }

  function handleGridDrop(event: DragEvent<HTMLDivElement>) {
    const payload = parseInternalDrag(event);
    if (!payload) return;
    event.preventDefault();
    void reorderToEnd(payload).catch((reason) => onFeedback(String(reason)));
  }

  return (
    <div
      className={`drawer-item-grid icon-size-${drawer.iconSize}`}
      ref={gridRef}
      data-dragging={dragging ?? undefined}
      onPointerDown={pointerDown}
      onPointerMove={pointerMove}
      onPointerUp={pointerUp}
      onPointerCancel={cancelPointer}
      onLostPointerCapture={() => { if (gesture.current) cancelPointer(); }}
      onKeyDown={(event) => { if (event.key === "Escape") cancelPointer(); }}
      onDragOver={(event) => {
        if (hasInternalDrag(event)) {
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
          dragging={dragging === item.id}
          dropTarget={dragging !== null && targetId === item.id && dragging !== item.id}
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
