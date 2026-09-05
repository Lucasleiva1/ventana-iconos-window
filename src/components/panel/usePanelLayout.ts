import { useLayoutEffect, useRef, useState } from "react";

/**
 * Escalones de tamaño de icono, del más grande al mínimo útil.
 * Por debajo de 24 px los iconos dejan de ser reconocibles, así que ese es el
 * piso: si el Panel se sigue achicando, aparece scroll en lugar de encogerlos.
 */
export const ICON_STEPS = [64, 56, 48, 40, 32, 24] as const;
export const MIN_ICON = ICON_STEPS[ICON_STEPS.length - 1];
export const MAX_ICON = ICON_STEPS[0];

/** Separación entre celdas, en píxeles lógicos. */
const GAP = 10;
/** Aire horizontal dentro de cada celda. */
const CELL_PADDING_X = 18;
/** Aire vertical dentro de cada celda, sin contar el texto. */
const CELL_PADDING_Y = 12;

export type LabelMode = "full" | "short" | "none";

export interface PanelLayout {
  icon: number;
  columns: number;
  cellWidth: number;
  cellHeight: number;
  labelMode: LabelMode;
  /** true cuando ya se llegó al icono mínimo y el contenido no entra. */
  scroll: boolean;
}

/**
 * Cuánto texto entra debajo del icono según su tamaño.
 * Grande: nombre completo en dos líneas. Mediano: una línea. Mínimo: sólo el
 * icono, con el nombre disponible en el tooltip.
 */
function labelModeFor(icon: number): LabelMode {
  if (icon >= 48) return "full";
  if (icon >= 32) return "short";
  return "none";
}

const LABEL_HEIGHT: Record<LabelMode, number> = { full: 30, short: 16, none: 0 };

function cellFor(icon: number) {
  const labelMode = labelModeFor(icon);
  return {
    labelMode,
    cellWidth: icon + CELL_PADDING_X,
    cellHeight: icon + CELL_PADDING_Y + LABEL_HEIGHT[labelMode],
  };
}

/**
 * Columnas que caben en `width` con celdas de `cellWidth`.
 * Debe coincidir con lo que hace CSS Grid con `repeat(auto-fill, ...)`, porque
 * el alto necesario se deduce de este número.
 */
function columnsFor(width: number, cellWidth: number) {
  return Math.max(1, Math.floor((width + GAP) / (cellWidth + GAP)));
}

/**
 * Elige el mayor tamaño de icono con el que **todo el contenido entra** en el
 * área disponible. Si ni siquiera el mínimo entra, se queda en el mínimo y
 * habilita scroll: los iconos nunca se vuelven microscópicos.
 */
export function computePanelLayout(
  width: number,
  height: number,
  count: number,
): PanelLayout {
  const usableWidth = Math.max(width, MIN_ICON + CELL_PADDING_X);
  const usableHeight = Math.max(height, 0);

  for (const icon of ICON_STEPS) {
    const { cellWidth, cellHeight, labelMode } = cellFor(icon);
    const columns = columnsFor(usableWidth, cellWidth);
    if (cellWidth > usableWidth && icon !== MIN_ICON) continue;
    const rows = count === 0 ? 0 : Math.ceil(count / columns);
    const needed = rows === 0 ? 0 : rows * cellHeight + (rows - 1) * GAP;
    if (needed <= usableHeight) {
      return { icon, columns, cellWidth, cellHeight, labelMode, scroll: false };
    }
  }

  const { cellWidth, cellHeight, labelMode } = cellFor(MIN_ICON);
  return {
    icon: MIN_ICON,
    columns: columnsFor(usableWidth, cellWidth),
    cellWidth,
    cellHeight,
    labelMode,
    scroll: true,
  };
}

export const PANEL_GRID_GAP = GAP;

/**
 * Observa el área de contenido del Panel y recalcula la distribución.
 *
 * Usa `ResizeObserver` (nunca polling) y agrupa las mediciones en un único
 * `requestAnimationFrame`, de modo que arrastrar un borde no dispara un render
 * por píxel. Tampoco escribe nada al guardado: el tamaño de icono se deriva del
 * tamaño del Panel y no se persiste.
 */
export function usePanelLayout(count: number) {
  const ref = useRef<HTMLDivElement>(null);
  const [layout, setLayout] = useState<PanelLayout>(() =>
    computePanelLayout(420, 240, count),
  );
  const frame = useRef(0);
  const previous = useRef({ width: -1, height: -1, count: -1 });

  useLayoutEffect(() => {
    const node = ref.current;
    if (!node) return;

    const measure = () => {
      const width = node.clientWidth;
      const height = node.clientHeight;
      if (
        width === previous.current.width
        && height === previous.current.height
        && count === previous.current.count
      ) {
        return;
      }
      previous.current = { width, height, count };
      setLayout(computePanelLayout(width, height, count));
    };

    const schedule = () => {
      if (frame.current) return;
      frame.current = window.requestAnimationFrame(() => {
        frame.current = 0;
        measure();
      });
    };

    const observer = new ResizeObserver(schedule);
    observer.observe(node);
    measure();

    return () => {
      observer.disconnect();
      if (frame.current) window.cancelAnimationFrame(frame.current);
      frame.current = 0;
    };
  }, [count]);

  return { gridRef: ref, layout };
}
