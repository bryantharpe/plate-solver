// The observing session belongs to the console. Browser preferences stay in settings.ts.
import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "./api";
import { isValidSite } from "./settings";

export interface SessionSite {
  lat_deg: number;
  lon_deg: number;
}

export interface ObservingSession {
  site: SessionSite | null;
  min_alt_deg?: number;
}

export interface SessionEdit {
  site?: SessionSite | null;
  min_alt_deg?: number;
}

const empty: ObservingSession = { site: null };

async function read(response: Response): Promise<ObservingSession> {
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body.error) message = body.error;
    } catch {
      // Keep the HTTP status if the response is not JSON.
    }
    throw new ApiError(message);
  }
  const body = (await response.json()) as Partial<ObservingSession> | null;
  // Older test fixtures (and an older console) can answer without session fields.
  return {
    site: body?.site && isValidSite(body.site.lat_deg, body.site.lon_deg) ? body.site : null,
    min_alt_deg: typeof body?.min_alt_deg === "number" ? body.min_alt_deg : undefined,
  };
}

export function useSession() {
  const [session, setSession] = useState<ObservingSession>(empty);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const live = useRef(false);
  const latest = useRef(0);

  const refresh = useCallback(async () => {
    const ticket = ++latest.current;
    try {
      const next = await read(await fetch("/api/session"));
      if (live.current && ticket === latest.current) {
        setSession(next);
        setLoaded(true);
        setError(null);
      }
    } catch (e) {
      if (live.current && ticket === latest.current) {
        setError(e instanceof ApiError ? e.message : "The observing session could not be loaded.");
      }
      throw e;
    }
  }, []);

  useEffect(() => {
    live.current = true;
    let timer: ReturnType<typeof setInterval> | undefined;
    const poll = () => void refresh().catch(() => {});
    const visibility = () => {
      clearInterval(timer);
      if (document.visibilityState === "visible") {
        poll();
        timer = setInterval(poll, 5000);
      }
    };
    visibility();
    document.addEventListener("visibilitychange", visibility);
    return () => {
      live.current = false;
      ++latest.current;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, [refresh]);

  const put = useCallback(
    async (edit: SessionEdit) => {
      // A poll started before this edit must not restore the old session afterwards.
      ++latest.current;
      const next = await read(
        await fetch("/api/session", {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(edit),
        }),
      );
      if (live.current) {
        setSession(next);
        setLoaded(true);
      }
      await refresh();
    },
    [refresh],
  );

  return { session, loaded, error, put };
}
