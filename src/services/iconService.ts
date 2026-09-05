import { drawerApi } from "./drawerApi";

const iconCache = new Map<string, Promise<string | null>>();

export class IconService {
  get(drawerId: string, itemId: string, iconKey: string) {
    const cached = iconCache.get(iconKey);
    if (cached) return cached;

    const icon = drawerApi.getItemIcon(drawerId, itemId).catch(() => null);
    iconCache.set(iconKey, icon);
    return icon;
  }

  getAtLevel(drawerId: string, relativePath: string, itemId: string, iconKey: string) {
    const cached = iconCache.get(iconKey);
    if (cached) return cached;

    const icon = drawerApi
      .getLevelItemIcon(drawerId, relativePath, itemId)
      .catch(() => null);
    iconCache.set(iconKey, icon);
    return icon;
  }
}

export const iconService = new IconService();
