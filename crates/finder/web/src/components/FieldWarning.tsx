import type { ObservingSession } from "../api";

export function FieldWarning({ field }: { field: ObservingSession["field"] }) {
  if (!field) return null;
  const { fov_deg: fov, db_min_fov_deg: min, db_max_fov_deg: max, unsolved_s } = field;
  const range = `${min}–${max}°`;
  const nearEdge = fov !== null && (fov <= min + 1 || fov >= max - 1);
  return (
    <div className="space-y-2 border-y border-rule py-3">
      <p>Camera field: {fov === null ? "not solved yet" : `${fov}°`}</p>
      <p className="quiet">Database range: {range}</p>
      {unsolved_s >= 30 ? (
        <p role="alert" className="text-ink">
          Frames with at least 10 stars have not solved for 30 s. The camera's field may be outside the database's{" "}
          {range} range; a matching database may be needed.
        </p>
      ) : nearEdge ? (
        <p role="alert" className="text-ink">
          The solved field is within 1° of an end of the database's range or outside it. A database built for the
          camera's field may be needed.
        </p>
      ) : null}
    </div>
  );
}
