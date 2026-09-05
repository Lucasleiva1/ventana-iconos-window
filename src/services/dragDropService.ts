import { getCurrentWindow, type DragDropEvent } from "@tauri-apps/api/window";

export type DrawerDropHandler = (event: DragDropEvent) => void;

export class DragDropService {
  listen(handler: DrawerDropHandler) {
    return getCurrentWindow().onDragDropEvent(({ payload }) => handler(payload));
  }
}

export const dragDropService = new DragDropService();
