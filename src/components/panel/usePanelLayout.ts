import { useLayoutEffect, useMemo, useRef, useState } from "react";
import type { PanelDensity, PanelIconMode } from "../../types/panel";

/**
 * Escalones de tamaño de icono, del máximo al mínimo útil.
 *
 * Por debajo de 24 px los iconos dejan de ser reconocibles, así que ese es el
 * piso: si el Panel se sigue achicando aparece scroll en lugar de encogerlos.
 * Por encima de 72 px sólo ocuparían espacio sin aportar nada, aunque el Panel
 * sea enorme.
 */
export const ICON_STEPS = [72, 64, 56, 48, 40, 32, 24] as const;
export const MIN_ICON = 24;
export const MAX_ICON = 72;

/** A partir de esta cantidad de accesos, el grid renderiza sólo lo visible. */
export const VIRTUALIZE_THRESHOLD = 240;
/** Filas extra que se dibujan fuera de la vista para que el scroll no parpadee. */
const OVERSCAN_ROWS = 4;

export type LabelMode = "full" | "short" | "none";

export interface DensityMetrics {
  gap: number;
  padding: number;
  cellPaddingX: number;
  cellPaddingY: number;
}

/** Debe coincidir con `PanelDensity::gap` y `::padding` del backend. */
const DENSITY: Record<PanelDensity, DensityMetrics> = {
  compact: { gap: 6, padding: 6, cellPaddingX: 10, cellPaddingY: 8 },
  normal: { gap: 10, padding: 10, cellPaddingX: 18, cellPaddingY: 12 },
  wide: { gap: 16, padding: 16, cellPaddingX: 26, cellPaddingY: 18 },
};

export function densityMetrics(density: PanelDensity) {
  return DENSITY[density] ?? DENSITY.normal;
}

export interface PanelLayout {
  icon: number;
  columns: number;
  cellWidth: number;
  cellHeight: number;
  rowHeight: number;
  labelMode: LabelMode;
  /** true cuando el contenido no entra y hay que desplazarse. */
  scroll: boolean;
  metrics: DensityMetrics;
}

/**
 * Cuánto texto entra debajo del icono.
 * Grande: nombre completo en dos líneas. Mediano: una línea. Mínimo: sólo el
 * icono, con el nombre disponible en el tooltip.
 */
function labelModeFor(icon: number): LabelMode {
  if (icon >= 48) return "full";
  if (icon >= 32) return "short";
  return "none";
}

const LABEL_HEIGHT: Record<LabelMode, number> = { full: 30, short: 16, none: 0 };

function cellFor(icon: number, metrics: DensityMetrics) {
  const labelMode = labelModeFor(icon);
  return {
    labelMode,
    cellWidth: icon + metrics.cellPaddingX,
    cellHeight: icon + metrics.cellPaddingY + LABEL_HEIGHT[labelMode],
  };
}

/**
 * Columnas que caben en `width`.
 * Replica lo que hace CSS Grid con `repeat(auto-fill, ...)`, porque el alto
 * necesario se deduce de este número.
 */
function columnsFor(width: number, cellWidth: number, gap: number) {
  return Math.max(1, Math.floor((width + gap) / (cellWidth + gap)));
}

function buildLayout(
  icon: number,
  metrics: DensityMetrics,
  width: number,
  scroll: boolean,
): PanelLayout {
  const { cellWidth, cellHeight, labelMode } = cellFor(icon, metrics);
  return {
    icon,
    columns: columnsFor(width, cellWidth, metrics.gap),
    cellWidth,
    cellHeight,
    rowHeight: cellHeight + metrics.gap,
    labelMode,
    scroll,
    metrics,
  };
}

function rowsNeeded(count: number, columns: number) {
  return count === 0 ? 0 : Math.ceil(count / columns);
}

function heightNeeded(rows: number, cellHeight: number, gap: number) {
  return rows === 0 ? 0 : rows * cellHeight + (rows - 1) * gap;
}

/**
 * Distribución del Panel.
 *
 * En modo automático elige el mayor tamaño de icono con el que **todo el
 * contenido entra**; si ni el mínimo entra, se queda en el mínimo y habilita
 * scroll. En modo manual respeta el tamaño elegido: nunca lo reduce, agrega
 * filas y scroll.
 */
export function computePanelLayout(
  width: number,
  height: number,
  count: number,
  density: PanelDensity,
  mode: PanelIconMode,
  manualIcon: number,
): PanelLayout {
  const metrics = densityMetrics(density);
  const usableWidth = Math.max(width, MIN_ICON + metrics.cellPaddingX);
  const usableHeight = Math.max(height, 0);

  if (mode === "manual") {
    const icon = Math.min(MAX_ICON, Math.max(MIN_ICON, Math.round(manualIcon)));
    const layout = buildLayout(icon, metrics, usableWidth, false);
    const rows = rowsNeeded(count, layout.columns);
    return {
      ...layout,
      scroll: heightNeeded(rows, layout.cellHeight, metrics.gap) > usableHeight,
    };
  }

  for (const icon of ICON_STEPS) {
    const { cellWidth, cellHeight } = cellFor(icon, metrics);
    if (cellWidth > usableWidth && icon !== MIN_ICON) continue;
    const columns = columnsFor(usableWidth, cellWidth, metrics.gap);
    const rows = rowsNeeded(count, columns);
    if (heightNeeded(rows, cellHeight, metrics.gap) <= usableHeight) {
      return buildLayout(icon, metrics, usableWidth, false);
    }
  }
  return buildLayout(MIN_ICON, metrics, usableWidth, true);
}

export interface VirtualWindow {
  from: number;
  to: number;
  padTop: number;
  padBottom: number;
}

/**
 * Rango de elementos que hace falta dibujar.
 *
 * Con pocos accesos devuelve todo, para que el arrastre y el reordenamiento
 * funcionen exactamente igual que siempre. Sólo cuando hay cientos recorta a
 * las filas visibles más un margen, y compensa con relleno arriba y abajo para
 * que la barra de scroll siga siendo fiel.
 */
export function computeVirtualWindow(
  count: number,
  scrollTop: number,
  viewportHeight: number,
  layout: PanelLayout,
): VirtualWindow {
  if (count <= VIRTUALIZE_THRESHOLD || layout.rowHeight <= 0) {
    return { from: 0, to: count, padTop: 0, padBottom: 0 };
  }
  const totalRows = rowsNeeded(count, layout.columns);
  const firstVisible = Math.floor(scrollTop / layout.rowHeight);
  const visibleRows = Math.ceil(viewportHeight / layout.rowHeight) + 1;
  const startRow = Math.max(0, firstVisible - OVERSCAN_ROWS);
  const endRow = Math.min(totalRows, firstVisible + visibleRows + OVERSCAN_ROWS);
  return {
    from: startRow * layout.columns,
    to: Math.min(count, endRow * layout.columns),
    padTop: startRow * layout.rowHeight,
    padBottom: Math.max(0, (totalRows - endRow) * layout.rowHeight),
  };
}

interface UsePanelLayoutOptions {
  count: number;
  density: PanelDensity;
  mode: PanelIconMode;
  manualIcon: number;
}

/**
 * Observa el área de contenido del Panel y recalcula la distribución.
 *
 * Usa `ResizeObserver` (nunca polling) y agrupa las mediciones en un único
 * `requestAnimationFrame`, de modo que arrastrar un borde no dispara un render
 * por píxel. Mide la caja de contenido, sin el padding, para que la grilla no
 * se pase del Panel. Y no escribe nada al guardado: el tamaño de icono se
 * deriva del tamaño del Panel y no se persiste.
 */
export function usePanelLayout({ count, density, mode, manualIcon }: UsePanelLayoutOptions) {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 420, height: 240 });
  const [scrollTop, setScrollTop] = useState(0);
  const frame = useRef(0);
  const scrollFrame = useRef(0);

  useLayoutEffect(() => {
    const node = ref.current;
    if (!node) return;

    const measure = (entry?: ResizeObserverEntry) => {
      // `contentRect` excluye el padding; `clientWidth` lo incluiría y la
      // grilla terminaría calculando más espacio del que realmente hay.
      const box = entry?.contentRect;
      const width = box?.width ?? node.clientWidth;
      const height = box?.height ?? node.clientHeight;
      setSize((previous) =>
        Math.abs(previous.width - width) < 0.5 && Math.abs(previous.height - height) < 0.5
          ? previous
          : { width, height },
      );
    };

    const observer = new ResizeObserver((entries) => {
      if (frame.current) return;
      frame.current = window.requestAnimationFrame(() => {
        frame.current = 0;
        measure(entries[0]);
      });
    });
    observer.observe(node);
    measure();

    return () => {
      observer.disconnect();
      if (frame.current) window.cancelAnimationFrame(frame.current);
      frame.current = 0;
    };
  }, []);

  const layout = useMemo(
    () => computePanelLayout(size.width, size.height, count, density, mode, manualIcon),
    [size.width, size.height, count, density, mode, manualIcon],
  );

  const virtual = useMemo(
    () => computeVirtualWindow(count, scrollTop, size.height, layout),
    [count, scrollTop, size.height, layout],
  );

  function onScroll(event: React.UIEvent<HTMLDivElement>) {
    if (count <= VIRTUALIZE_THRESHOLD) return;
    const next = event.currentTarget.scrollTop;
    if (scrollFrame.current) return;
    scrollFrame.current = window.requestAnimationFrame(() => {
      scrollFrame.current = 0;
      setScrollTop(next);
    });
  }

  return { gridRef: ref, layout, virtual, onScroll };
}
