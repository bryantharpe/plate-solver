import { useEffect, useState } from "react";
import { putNodeCamera, type LiveReport, type ObservingSession } from "../api";

export function ScopeNodeSection({ session, live }: { session: ObservingSession; live: LiveReport | null }) {
  const [address, setAddress] = useState(session.node?.address ?? "");
  const [token, setToken] = useState("");
  const [tokenSet, setTokenSet] = useState(session.node?.token_set ?? false);
  const [exposure, setExposure] = useState(String(session.camera_settings?.exposure_ms ?? "auto"));
  const [gain, setGain] = useState(String(session.camera_settings?.gain ?? "auto"));
  const [sigma, setSigma] = useState(String(session.camera_settings?.sigma ?? 8));
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  // Polls and live frames must leave drafts alone unless a stored value changed.
  useEffect(() => setAddress(session.node?.address ?? ""), [session.node?.address]);
  useEffect(() => setTokenSet(session.node?.token_set ?? false), [session.node?.token_set]);
  useEffect(
    () => setExposure(String(session.camera_settings?.exposure_ms ?? "auto")),
    [session.camera_settings?.exposure_ms],
  );
  useEffect(() => setGain(String(session.camera_settings?.gain ?? "auto")), [session.camera_settings?.gain]);
  useEffect(() => setSigma(String(session.camera_settings?.sigma ?? 8)), [session.camera_settings?.sigma]);

  const save = async () => {
    const automatic = (value: string): "auto" | number =>
      value.trim().toLowerCase() === "auto" ? "auto" : Number(value);
    const exposure_ms = automatic(exposure);
    const gainValue = automatic(gain);
    const threshold = sigma.trim().toLowerCase() === "auto" ? 8 : Number(sigma);
    setFailed(false);
    if (
      !exposure.trim() ||
      !gain.trim() ||
      !sigma.trim() ||
      (exposure_ms !== "auto" && (!Number.isFinite(exposure_ms) || exposure_ms <= 0)) ||
      (gainValue !== "auto" && (!Number.isFinite(gainValue) || gainValue <= 0)) ||
      !Number.isFinite(threshold) ||
      threshold < 1 ||
      threshold > 50
    ) {
      setFailed(true);
      setMessage(
        "Exposure and gain must be auto or positive numbers. Detection threshold must be 1–50 σ or auto (8 σ).",
      );
      return;
    }
    setSaving(true);
    setMessage("Saving…");
    try {
      const next = await putNodeCamera({
        ...(address.trim() || token
          ? { node: { ...(address.trim() && { address: address.trim() }), ...(token && { token }) } }
          : {}),
        camera_settings: { exposure_ms, gain: gainValue, sigma: threshold },
      });
      setToken("");
      setTokenSet(next.node?.token_set ?? false);
      setMessage("Scope node and camera saved.");
    } catch (error) {
      setFailed(true);
      // Never echo a pairing token, even if a failed request's message contains it.
      const reason = error instanceof Error ? error.message : "Could not save the scope node and camera.";
      setMessage(token ? reason.split(token).join("****") : reason);
    } finally {
      setSaving(false);
    }
  };
  const node = session.node;
  const stars = node?.connected && live ? live.stars : node?.stars;
  return (
    <section aria-labelledby="scope-node-title" className="space-y-3">
      <h2 id="scope-node-title" className="section-title">
        Scope node and camera
      </h2>
      <div className="space-y-1 text-ink-2">
        <p>{node?.connected ? "Connected" : "Disconnected"}</p>
        {node?.camera && <p>{node.camera.model}</p>}
        {stars != null && <p>{stars} stars</p>}
      </div>
      {node?.out_of_range.map((range) => (
        <p key={range.setting} role="alert" className="text-ink">
          {range.setting === "exposure_ms" ? "Exposure" : "Gain"} {range.stored}
          {range.setting === "exposure_ms" ? " ms" : ""} is outside the camera's range {range.min}–{range.max}
          {range.setting === "exposure_ms" ? " ms" : ""}. Using automatic on the camera; the stored value is kept.
        </p>
      ))}
      <form
        className="space-y-3 border-y border-rule py-4"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <NodeField label="Scope node address" value={address} onChange={setAddress} placeholder="finder.local:50051" />
        <NodeField label="Token" value={token} onChange={setToken} password />
        <p className="quiet">
          {tokenSet
            ? "A token is set. Leave empty to keep it."
            : "No token is set. Enter the scope node's token to pair."}
        </p>
        <div className="grid grid-cols-2 gap-3">
          <NodeField label="Exposure" value={exposure} onChange={setExposure} unit="ms" />
          <NodeField label="Gain" value={gain} onChange={setGain} />
        </div>
        <p className="quiet">Exposure and gain: auto or a positive value.</p>
        <NodeField label="Detection threshold" value={sigma} onChange={setSigma} unit="σ" />
        <p className="quiet">1–50 σ, or auto for cedar's 8 σ.</p>
        {message && (
          <p role={failed ? "alert" : "status"} className="text-ink">
            {message}
          </p>
        )}
        <button type="submit" className="button button-primary" disabled={saving}>
          Save scope node and camera
        </button>
      </form>
    </section>
  );
}

function NodeField({
  label,
  value,
  onChange,
  unit,
  placeholder,
  password = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  unit?: string;
  placeholder?: string;
  password?: boolean;
}) {
  return (
    <label className="block space-y-1">
      <span className="text-sm font-semibold text-ink-2">{label}</span>
      <span className="flex items-center border border-ink focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-ink">
        <input
          aria-label={label}
          type={password ? "password" : "text"}
          autoComplete={password ? "new-password" : "off"}
          spellCheck={false}
          autoCapitalize="none"
          value={value}
          placeholder={placeholder}
          onChange={(event) => onChange(event.target.value)}
          className="h-11 w-full min-w-0 bg-transparent px-3 text-base outline-none"
        />
        {unit && (
          <span aria-hidden="true" className="pr-3 text-sm text-ink-2">
            {unit}
          </span>
        )}
      </span>
    </label>
  );
}
