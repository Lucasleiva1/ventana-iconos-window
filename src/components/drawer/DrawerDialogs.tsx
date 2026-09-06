import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type ReactNode,
} from "react";

/**
 * Diálogos propios del Cajón. Los de Windows (`prompt` y `confirm`) muestran la
 * dirección del servidor, no respetan el estilo de la aplicación y bloquean la
 * ventana entera, así que acá se reemplazan por un cuadro con el mismo aspecto
 * que el resto del Cajón.
 */

interface AskTextOptions {
  title: string;
  label: string;
  defaultValue?: string;
  confirmLabel?: string;
  placeholder?: string;
}

interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
}

interface DrawerDialogsApi {
  /** Devuelve el texto ya recortado, o null si se canceló o quedó vacío. */
  askText: (options: AskTextOptions) => Promise<string | null>;
  confirm: (options: ConfirmOptions) => Promise<boolean>;
}

type PendingDialog =
  | { kind: "text"; options: AskTextOptions; resolve: (value: string | null) => void }
  | { kind: "confirm"; options: ConfirmOptions; resolve: (value: boolean) => void };

const DrawerDialogsContext = createContext<DrawerDialogsApi | null>(null);

export function useDrawerDialogs(): DrawerDialogsApi {
  const api = useContext(DrawerDialogsContext);
  if (!api) throw new Error("useDrawerDialogs necesita estar dentro de DrawerDialogsProvider.");
  return api;
}

export function DrawerDialogsProvider({ children }: { children: ReactNode }) {
  const [pending, setPending] = useState<PendingDialog | null>(null);
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const api = useMemo<DrawerDialogsApi>(() => ({
    askText: (options) => new Promise((resolve) => {
      setValue(options.defaultValue ?? "");
      setPending({ kind: "text", options, resolve });
    }),
    confirm: (options) => new Promise((resolve) => {
      setPending({ kind: "confirm", options, resolve });
    }),
  }), []);

  const close = useCallback((result: string | null | boolean) => {
    setPending((current) => {
      if (!current) return null;
      if (current.kind === "text") current.resolve(typeof result === "string" ? result : null);
      else current.resolve(result === true);
      return null;
    });
  }, []);

  useEffect(() => {
    if (!pending) return;
    inputRef.current?.focus();
    inputRef.current?.select();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close(pending.kind === "text" ? null : false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [pending, close]);

  function submitText(event: FormEvent) {
    event.preventDefault();
    const trimmed = value.trim();
    close(trimmed ? trimmed : null);
  }

  return (
    <DrawerDialogsContext.Provider value={api}>
      {children}
      {pending && (
        <div
          className="drawer-dialog-backdrop"
          role="presentation"
          onPointerDown={(event) => {
            if (event.target === event.currentTarget) close(pending.kind === "text" ? null : false);
          }}
        >
          <div
            className={`drawer-dialog ${pending.kind === "confirm" && pending.options.danger ? "is-danger" : ""}`}
            role="dialog"
            aria-modal="true"
            aria-label={pending.options.title}
          >
            <p className="drawer-dialog-title">{pending.options.title}</p>
            {pending.kind === "text" ? (
              <form onSubmit={submitText}>
                <label className="drawer-dialog-label" htmlFor="drawer-dialog-input">
                  {pending.options.label}
                </label>
                <input
                  id="drawer-dialog-input"
                  ref={inputRef}
                  value={value}
                  maxLength={120}
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={pending.options.placeholder}
                  onChange={(event) => setValue(event.target.value)}
                />
                <div className="drawer-dialog-actions">
                  <button type="button" className="drawer-dialog-button" onClick={() => close(null)}>
                    Cancelar
                  </button>
                  <button type="submit" className="drawer-dialog-button is-primary" disabled={!value.trim()}>
                    {pending.options.confirmLabel ?? "Aceptar"}
                  </button>
                </div>
              </form>
            ) : (
              <>
                <p className="drawer-dialog-message">{pending.options.message}</p>
                <div className="drawer-dialog-actions">
                  <button type="button" className="drawer-dialog-button" onClick={() => close(false)}>
                    Cancelar
                  </button>
                  <button
                    type="button"
                    className="drawer-dialog-button is-primary"
                    onClick={() => close(true)}
                  >
                    {pending.options.confirmLabel ?? "Aceptar"}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </DrawerDialogsContext.Provider>
  );
}
