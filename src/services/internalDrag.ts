import type { DragEvent } from "react";
import type { InternalDragPayload } from "../types/drawer";

export const INTERNAL_ITEM_MIME = "application/x-desktop-organizer-item";

export function hasInternalDrag(event: DragEvent) {
  return event.dataTransfer.types.includes(INTERNAL_ITEM_MIME)
    || event.dataTransfer.types.includes("text/plain");
}

export function parseInternalDrag(event: DragEvent): InternalDragPayload | null {
  const raw = event.dataTransfer.getData(INTERNAL_ITEM_MIME)
    || event.dataTransfer.getData("text/plain");
  try {
    const value = JSON.parse(raw) as Partial<InternalDragPayload>;
    if (value.sourceDrawerId && value.itemId && value.sourceRelativePath !== undefined) {
      return value as InternalDragPayload;
    }
  } catch {
    return null;
  }
  return null;
}

export function joinDrawerPath(parent: string, child: string) {
  return [parent.replace(/[\\/]+$/, ""), child.replace(/^[\\/]+/, "")]
    .filter(Boolean)
    .join("\\");
}
