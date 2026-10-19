// Set the observing site, so the page can say what is up.

import { useEffect, useRef, useState } from "react";
import { isValidSite, type Site } from "../settings";

interface Props {
  open: boolean;
  site: Site | null;
  onClose: () => void;
  onSave: (site: Site | null) => void;
}

export function SiteDialog({ open, site, onClose, onSave }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [lat, setLat] = useState("");
  const [lon, setLon] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    const d = dialog.current;
    if (!d) return;
    if (open && !d.open) {
      setLat(site ? String(site.lat) : "");
      setLon(site ? String(site.lon) : "");
      setMessage(null);
      d.showModal();
    }
    if (!open && d.open) d.close();
  }, [open, site]);

  const save = () => {
    const la = Number(lat);
    const lo = Number(lon);
    if (lat.trim() === "" || lon.trim() === "" || !isValidSite(la, lo)) {
      setMessage("Latitude goes from −90 to 90 and longitude from −180 to 180, east positive.");
      return;
    }
    onSave({ lat: la, lon: lo });
    onClose();
  };

  const locate = () => {
    if (!window.isSecureContext || !navigator.geolocation) {
      setMessage(
        "This browser only shares its location with pages served over HTTPS or from the same device. Enter it by hand.",
      );
      return;
    }
    setMessage("Asking for this device's location…");
    navigator.geolocation.getCurrentPosition(
      (p) => {
        setLat(p.coords.latitude.toFixed(3));
        setLon(p.coords.longitude.toFixed(3));
        setMessage(null);
      },
      (e) => setMessage(`No location: ${e.message}. Enter it by hand.`),
      { timeout: 15000 },
    );
  };

  return (
    <dialog ref={dialog} className="sheet max-w-md" aria-labelledby="site-title" onClose={onClose}>
      <header className="flex items-center justify-between border-b border-rule px-4 py-3 sm:px-5">
        <h2 id="site-title" className="section-title">
          Observing site
        </h2>
        <button type="button" className="button button-quiet" onClick={onClose}>
          Close
        </button>
      </header>
      <form
        className="space-y-4 px-4 py-4 sm:px-5"
        onSubmit={(e) => {
          e.preventDefault();
          save();
        }}
      >
        <p className="quiet">Used only on this device, to work out what is above your horizon now.</p>
        <div className="grid grid-cols-2 gap-3">
          <Coordinate label="Latitude" unit="° N" value={lat} onChange={setLat} placeholder="40.0" />
          <Coordinate label="Longitude" unit="° E" value={lon} onChange={setLon} placeholder="-75.0" />
        </div>
        {message && (
          <p className="text-[0.9375rem] text-ink" role="status">
            {message}
          </p>
        )}
        <div className="flex flex-wrap justify-end gap-3 pt-1">
          <button type="button" className="button" onClick={locate}>
            Use this device's location
          </button>
          <button type="submit" className="button button-primary">
            Save site
          </button>
        </div>
        {site && (
          <button
            type="button"
            className="link"
            onClick={() => {
              onSave(null);
              onClose();
            }}
          >
            Forget the site
          </button>
        )}
      </form>
    </dialog>
  );
}

function Coordinate({
  label,
  unit,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  unit: string;
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
}) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-semibold text-ink-2">{label}</span>
      <span className="flex items-center border border-ink focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
        <input
          inputMode="decimal"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="h-11 w-full min-w-0 bg-transparent px-3 text-base outline-none"
        />
        <span className="pr-3 text-sm text-ink-2">{unit}</span>
      </span>
    </label>
  );
}
