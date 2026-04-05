import { useEffect, useRef, useState } from "react";
import { fetchSession, fetchStatus, triggerRerun as triggerServerRerun } from "../utils/api.js";
import { DEFAULT_SERVER_STATUS, POLL_INTERVAL_MS } from "../utils/constants.js";

/**
 * Hook for session fetching and polling.
 * @returns {{ session: object|null, serverStatus: object, error: string, triggerRerun: () => Promise<void> }}
 */
export function useSession() {
  const [session, setSession] = useState(null);
  const [error, setError] = useState("");
  const [serverStatus, setServerStatus] = useState(DEFAULT_SERVER_STATUS);

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
        setServerStatus(status);

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
  }, []);

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
