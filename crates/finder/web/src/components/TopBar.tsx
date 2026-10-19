// The top bar: the frame being charted, the observing site, and night mode.

import { thumbUrl, type FrameSource } from "../api";
import type { Site } from "../settings";
import { Chevron, Pin, Reticle, Torch } from "./Icons";

interface Props {
  source: FrameSource | null;
  label: string | null;
  site: Site | null;
  night: boolean;
  onFrame: () => void;
  onSite: () => void;
  onNight: (on: boolean) => void;
}

const siteText = (site: Site) => `${Math.abs(site.lat).toFixed(1)}° ${site.lat >= 0 ? "N" : "S"}`;

export function TopBar({ source, label, site, night, onFrame, onSite, onNight }: Props) {
  return (
    <header className="sticky top-0 z-10 border-b border-rule bg-paper">
      <div className="mx-auto flex h-16 max-w-[90rem] items-center gap-2 px-3 sm:gap-3 sm:px-5">
        <a
          href="/"
          className="mr-auto flex items-center gap-2 font-semibold tracking-[0.02em]"
          aria-label="Finder home"
        >
          <Reticle size={22} />
          <span className="hidden text-[1.05rem] min-[400px]:inline">Finder</span>
        </a>
        <button type="button" className="chip max-w-[44vw] pl-1.5" onClick={onFrame} aria-haspopup="dialog">
          {source?.kind === "sample" ? (
            <img src={thumbUrl(source.name)} alt="" className="photo size-7 object-cover" />
          ) : (
            <span className="grid size-7 place-items-center bg-paper-2" aria-hidden="true">
              <Reticle size={14} />
            </span>
          )}
          <span className="truncate">{label ?? "Choose a frame"}</span>
          <Chevron size={16} />
        </button>
        <button
          type="button"
          className="chip px-2.5"
          onClick={onSite}
          aria-haspopup="dialog"
          aria-label={site ? `Site ${siteText(site)}` : "Set your site"}
        >
          <Pin size={17} />
          <span className="hidden sm:inline">{site ? siteText(site) : "Set site"}</span>
          {site && <span className="sm:hidden">{Math.round(site.lat)}°</span>}
        </button>
        <label className="flex items-center gap-2 text-sm font-medium">
          <span className="hidden sm:inline">Night</span>
          <span className="sm:hidden" aria-hidden="true">
            <Torch size={18} />
          </span>
          <button
            type="button"
            role="switch"
            aria-checked={night}
            aria-label="Night mode: red and black"
            className="switch"
            onClick={() => onNight(!night)}
          />
        </label>
      </div>
    </header>
  );
}
