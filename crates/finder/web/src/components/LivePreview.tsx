/** Occasional node photographs stay below the chart and take its night filter. */
export function LivePreview({ seq }: { seq: number | null }) {
  if (seq === null) return null;
  return (
    <div className="mt-4 border border-ink bg-photo">
      <img
        src={`/api/live/preview.jpg?seq=${seq}`}
        alt={`Live preview, frame ${seq}`}
        className="photo block h-auto w-full"
      />
    </div>
  );
}
