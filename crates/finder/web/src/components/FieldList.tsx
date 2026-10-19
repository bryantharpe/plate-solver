// What is in the field: the deep-sky objects on the frame, and the named stars matched.

import type { ObjectMark, SolveReport } from "../api";
import { mag, objectTitle } from "../format";
import { FamilyIcon } from "./Symbol";

interface Props {
  report: SolveReport;
  selected: string | null;
  onSelect: (id: string) => void;
}

export function FieldList({ report, selected, onSelect }: Props) {
  const named = report.stars.filter((s) => s.matched && s.name);
  const objects = report.objects;
  return (
    <section aria-labelledby="field-title" className="space-y-2">
      <h2 id="field-title" className="section-title">
        In the field
      </h2>
      {objects.length === 0 ? (
        <p className="quiet">No Messier objects in this frame.</p>
      ) : (
        <ul className="divide-y divide-rule border-y border-rule">
          {objects.map((o) => (
            <li key={o.id}>
              <ObjectRow object={o} selected={selected === o.id} onSelect={() => onSelect(o.id)} />
            </li>
          ))}
        </ul>
      )}
      {named.length > 0 && (
        <p className="flex flex-wrap gap-x-3 gap-y-1 pt-1 text-[0.9375rem]">
          <span className="font-semibold text-ink-2">Stars</span>
          {named.map((s, i) => (
            <span key={s.id}>
              {s.name}
              {i < named.length - 1 && (
                <span className="ml-3 text-ink-3" aria-hidden="true">
                  ·
                </span>
              )}
            </span>
          ))}
        </p>
      )}
    </section>
  );
}

function ObjectRow({ object: o, selected, onSelect }: { object: ObjectMark; selected: boolean; onSelect: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className="group grid w-full grid-cols-[1.5rem_3.25rem_1fr_auto] items-center gap-x-2 py-2.5 text-left transition-colors hover:bg-paper-2 aria-pressed:bg-paper-2"
    >
      <span className={`icon-${o.family} justify-self-center`}>
        <FamilyIcon family={o.family} size={18} />
      </span>
      <span className="font-semibold italic">{o.id}</span>
      <span className="min-w-0">
        <span className="block truncate">{objectTitle(o)}</span>
        <span className="block text-sm text-ink-2">
          {o.kind}
          {!o.inside && " · at the edge"}
        </span>
      </span>
      <span className="text-right text-sm text-ink-2">
        mag <span className="text-base text-ink">{mag(o.mag)}</span>
      </span>
    </button>
  );
}
