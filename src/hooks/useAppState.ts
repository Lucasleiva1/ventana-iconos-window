import { useCallback, useEffect, useState } from "react";
import { drawerApi } from "../services/drawerApi";
import type { PersistedState } from "../types/drawer";

export function useAppState() {
  const [state, setState] = useState<PersistedState | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setState(await drawerApi.getState());
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void refresh();
    void drawerApi.onStateChanged((nextState) => {
      if (active) setState(nextState);
    }).then((stopListening) => {
      if (active) unlisten = stopListening;
      else stopListening();
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [refresh]);

  return { state, error, refresh };
}

