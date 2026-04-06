import { useEffect, useRef } from "react";
import { fetchSession, fetchStatus, triggerRerun as triggerServerRerun } from "../utils/api.js";
import { POLL_INTERVAL_MS } from "../utils/constants.js";
import { useSessionStore } from "../store/sessionStore.js";

function isServerStatusEqual(left, right) {
  return (
    left.running === right.running &&
    left.can_rerun === right.can_rerun &&
    left.generation === right.generation &&
    left.session_title === right.session_title &&
    left.last_error === right.last_error
  );
}

/**
 * Hook for session fetching and polling.
 * Manages session state in zustand store.
 */
export function useSession() {
  const session = useSessionStore((state) => state.session);
  const serverStatus = useSessionStore((state) => state.serverStatus);
  const error = useSessionStore((state) => state.error);
  const setSession = useSessionStore((state) => state.setSession);
  const setServerStatus = useSessionStore((state) => state.setServerStatus);
  const setError = useSessionStore((state) => state.setError);

  const generationRef = useRef(0);
  const sessionRef = useRef(null);

  useEffect(() => {
    sessionRef.current = session;
  }, [session]);

  useEffect(() => {
    let cancelled = false;

    const refresh = async (forceSessionLoad = false) => {
      try {
        const status = await fetchStatus();
        if (cancelled) {
          return;
        }
        setServerStatus((current) =>
          isServerStatusEqual(current, status) ? current : status
        );

        const generationChanged = status.generation !== generationRef.current;
        if (!forceSessionLoad && !generationChanged && sessionRef.current) {
          return;
        }

        const nextSession = await fetchSession();
        if (cancelled) {
          return;
        }

        generationRef.current = status.generation;
        setSession(nextSession);
        setError("");

        return { generationChanged, hadPreviousSession: !!sessionRef.current };
      } catch (fetchError) {
        if (!cancelled && !sessionRef.current) {
          setError(fetchError.message);
        }
        return null;
      }
    };

    refresh(true);
    const timer = window.setInterval(() => {
      void refresh(false);
    }, POLL_INTERVAL_MS);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [setSession, setServerStatus, setError]);

  const triggerRerun = async () => {
    if (!serverStatus.can_rerun) {
      return;
    }

    setServerStatus((current) => ({
      ...current,
      running: true,
      last_error: null
    }));

    try {
      const nextStatus = await triggerServerRerun();
      setServerStatus(nextStatus);
    } catch (requestError) {
      setServerStatus((current) => ({
        ...current,
        running: false,
        last_error: requestError.message
      }));
    }
  };

  return { session, serverStatus, error, triggerRerun, setServerStatus };
}
