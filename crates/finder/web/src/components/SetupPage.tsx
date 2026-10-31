import { useEffect, useState } from "react";
import { isValidSite } from "../settings";
import type { ObservingSession, SessionEdit } from "../useSession";

interface Props {
  session: ObservingSession;
  onSave: (edit: SessionEdit) => Promise<void>;
  onBack: () => void;
}

export function SetupPage({ session, onSave, onBack }: Props) {
  const [lat, setLat] = useState("");
  const [lon, setLon] = useState("");
  const [minimum, setMinimum] = useState("15");
  const [siteMessage, setSiteMessage] = useState<string | null>(null);
  const [minimumMessage, setMinimumMessage] = useState<string | null>(null);
  const [savingSite, setSavingSite] = useState(false);
  const [savingMinimum, setSavingMinimum] = useState(false);

  // Polls return new objects. Reset fields only when their shared values change.
  useEffect(() => {
    setLat(session.site ? String(session.site.lat_deg) : "");
    setLon(session.site ? String(session.site.lon_deg) : "");
  }, [session.site?.lat_deg, session.site?.lon_deg]);
  useEffect(() => setMinimum(String(session.min_alt_deg ?? 15)), [session.min_alt_deg]);

  const save = async (
    edit: SessionEdit,
    message: string,
    setMessage: (message: string) => void,
    setSaving: (saving: boolean) => void,
  ) => {
    setSaving(true);
    try {
      await onSave(edit);
      setMessage(message);
    } catch (e) {
      setMessage(e instanceof Error ? e.message : "The observing session could not be saved. Try again.");
    } finally {
      setSaving(false);
    }
  };

  const saveSite = () => {
    const la = Number(lat);
    const lo = Number(lon);
    if (lat.trim() === "" || lon.trim() === "" || !isValidSite(la, lo)) {
      setSiteMessage("Latitude goes from −90 to 90 and longitude from −180 to 180, east positive.");
      return;
    }
    void save({ site: { lat_deg: la, lon_deg: lo } }, "Site saved.", setSiteMessage, setSavingSite);
  };

  const saveMinimum = () => {
    const value = Number(minimum);
    if (minimum.trim() === "" || !Number.isFinite(value) || value < 0 || value > 80) {
      setMinimumMessage("Minimum altitude must be 0–80°.");
      return;
    }
    void save({ min_alt_deg: value }, "Minimum altitude saved.", setMinimumMessage, setSavingMinimum);
  };

  const locate = () => {
    if (!window.isSecureContext || !navigator.geolocation) {
      setSiteMessage(
        "This browser only shares its location with pages served over HTTPS or from the same device. Enter it by hand.",
      );
      return;
    }
    setSiteMessage("Asking for this device's location…");
    navigator.geolocation.getCurrentPosition(
      (p) => {
        setLat(p.coords.latitude.toFixed(3));
        setLon(p.coords.longitude.toFixed(3));
        setSiteMessage(null);
      },
      (e) => setSiteMessage(`No location: ${e.message}. Enter it by hand.`),
      { timeout: 15000 },
    );
  };

  return (
    <main className="mx-auto max-w-[34rem] space-y-6 px-4 pt-4 pb-16 sm:px-5 sm:pt-6">
      <header className="border border-ink p-[3px]">
        <div className="space-y-3 border border-ink p-4">
          <h1 className="chart-title text-[1.3rem]">Setup</h1>
          <p className="quiet">One observing session, shared by every phone and laptop connected to the console.</p>
          <a
            href="/"
            className="link inline-flex min-h-11 items-center"
            onClick={(e) => {
              e.preventDefault();
              onBack();
            }}
          >
            Back to console
          </a>
        </div>
      </header>

      <section aria-labelledby="setup-site-title" className="space-y-3">
        <h2 id="setup-site-title" className="section-title">
          Site
        </h2>
        {!session.site && <p className="quiet">Not set. Set the site to see what is up now.</p>}
        <form
          className="space-y-3 border-y border-rule py-3"
          onSubmit={(e) => {
            e.preventDefault();
            saveSite();
          }}
        >
          <div className="grid grid-cols-2 gap-3">
            <Field id="setup-lat" label="Latitude" unit="° N" value={lat} onChange={setLat} />
            <Field id="setup-lon" label="Longitude" unit="° E" value={lon} onChange={setLon} />
          </div>
          <div className="flex flex-wrap gap-3">
            <button type="button" className="button" onClick={locate} disabled={savingSite}>
              Use this device's location
            </button>
            <button type="submit" className="button button-primary" disabled={savingSite}>
              Save site
            </button>
          </div>
          {session.site && (
            <button
              type="button"
              className="link inline-flex min-h-11 items-center"
              disabled={savingSite}
              onClick={() => void save({ site: null }, "Site forgotten.", setSiteMessage, setSavingSite)}
            >
              Forget the site
            </button>
          )}
          {siteMessage && (
            <p role="status" className="quiet">
              {siteMessage}
            </p>
          )}
        </form>
      </section>

      <section aria-labelledby="setup-minimum-title" className="space-y-3">
        <h2 id="setup-minimum-title" className="section-title">
          Minimum altitude
        </h2>
        <p className="quiet">Targets below this are too low; below 0° they are below the horizon.</p>
        <form
          className="space-y-3 border-y border-rule py-3"
          onSubmit={(e) => {
            e.preventDefault();
            saveMinimum();
          }}
        >
          <div className="max-w-44">
            <Field id="setup-minimum" label="Minimum altitude" unit="°" value={minimum} onChange={setMinimum} />
          </div>
          <button type="submit" className="button button-primary" disabled={savingMinimum}>
            Save minimum altitude
          </button>
          {minimumMessage && (
            <p role="status" className="quiet">
              {minimumMessage}
            </p>
          )}
        </form>
      </section>
    </main>
  );
}

function Field({
  id,
  label,
  unit,
  value,
  onChange,
}: {
  id: string;
  label: string;
  unit: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="space-y-1">
      <label htmlFor={id} className="block text-sm font-semibold text-ink-2">
        {label}
      </label>
      <div className="flex items-center border border-ink focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
        <input
          id={id}
          inputMode="decimal"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="h-11 w-full min-w-0 bg-transparent px-3 text-base outline-none"
        />
        <span className="pr-3 text-sm whitespace-nowrap text-ink-2">{unit}</span>
      </div>
    </div>
  );
}
